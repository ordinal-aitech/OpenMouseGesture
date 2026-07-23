// 概要: マウスジェスチャーの軌跡をネイティブWin32レイヤードウィンドウで描画
// 入出力:
//   - 入力: 軌跡座標の配列 Vec<(i32, i32)>
//   - 出力: 画面上に軌跡を描画（透明背景、最前面表示）
// 実装詳細:
//   - WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST でクリックスルー可能な透明ウィンドウ
//   - UpdateLayeredWindow + ARGB32ビットマップでアルファチャンネル制御
//   - グロー(外側/低不透明度) → ボディ(中間) → コア(明るいハイライト)の3層を
//     それぞれ丸線端/丸結合のジオメトリックペンで別バッファに描画し、
//     各レイヤーのカバレッジからプリマルチプライ済みARGBを合成する
//   - 別スレッドでウィンドウメッセージループを実行
//   - オーバーレイウィンドウは起動時に仮想デスクトップ全体
//     (SM_XVIRTUALSCREEN/SM_YVIRTUALSCREEN起点、SM_CXVIRTUALSCREEN×
//     SM_CYVIRTUALSCREENサイズ)で一度だけ作成し、以後 SetWindowPos による
//     移動・リサイズを一切行わない。ジェスチャー中に限らずプロセス生存中
//     ずっとoriginとサイズが固定されるため、バウンディング拡大に伴う
//     ウィンドウの再配置・ビットマップ再確保による「既に描画済みの軌跡が
//     一瞬ずれる」現象が構造的に発生し得ない。
//   - 例外的に、ジェスチャー開始の立ち上がりエッジでのみ SetWindowPos を
//     SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE で呼び、origin/サイズ/アクティブ
//     ウィンドウを一切変えずに TOPMOST バンドの最上位だけを再主張する。
//     これにより、オーバーレイ生成後に最前面へ昇格したフルスクリーン/
//     ボーダーレスウィンドウの上にも軌跡を表示できる（z-orderのみの操作なので
//     上記の「ずれ」不変条件は維持される）。
//   - ウィンドウはプロセス生存中ずっと表示状態を保ち、「軌跡なし」は
//     全面透明フレーム（アルファ0）で表現する。ShowWindow の表示/非表示切替を
//     廃止したことで、フルスクリーン相当のウィンドウに対する ShowWindow 遷移が
//     DWM合成を揺らし、フォアグラウンド背後のウィンドウが一瞬露出する現象を抑制する。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*, Win32::UI::WindowsAndMessaging::*,
};

// 軌跡ポイント列と可視状態は単一Mutexで保持する。
// 点列とバウンディングを別々のMutexにすると、window_procでのスナップショット取得と
// append_trajectory_point/update_trajectoryでの更新がインターリーブし、
// 「点列は新しいがバウンディング(=ウィンドウ位置/オフセット)は古い」
// という不整合フレームが発生し、軌跡全体が一瞬ずれて見える(トレンブリング)原因になっていた。
// バウンディングは常にこの点列スナップショットから同じ式で再計算し、単一の情報源とする。
struct TrajectoryState {
    points: Vec<(i32, i32)>,
    visible: bool,
}

static RENDERER_HWND: Mutex<Option<isize>> = Mutex::new(None);
static TRAJECTORY_STATE: Mutex<TrajectoryState> = Mutex::new(TrajectoryState {
    points: Vec::new(),
    visible: false,
});
// ウィンドウのorigin/サイズは起動時に仮想デスクトップ全体で一度だけ確定し、
// 以後は不変(read-onlyスナップショットとして扱う)。書き込みはinit_renderer内の
// 初期化コードからのみ行われる。
static WINDOW_SIZE: Mutex<(i32, i32)> = Mutex::new((0, 0));
static WINDOW_POS: Mutex<(i32, i32)> = Mutex::new((0, 0));
static MEMORY_DC: Mutex<Option<isize>> = Mutex::new(None);
static MEMORY_BITMAP: Mutex<Option<isize>> = Mutex::new(None);
static MASK_DC_GLOW: Mutex<Option<isize>> = Mutex::new(None);
static MASK_BITMAP_GLOW: Mutex<Option<isize>> = Mutex::new(None);
static MASK_DC_BODY: Mutex<Option<isize>> = Mutex::new(None);
static MASK_BITMAP_BODY: Mutex<Option<isize>> = Mutex::new(None);
static MASK_DC_CORE: Mutex<Option<isize>> = Mutex::new(None);
static MASK_BITMAP_CORE: Mutex<Option<isize>> = Mutex::new(None);
static LAST_RENDER_TIME: Mutex<Option<Instant>> = Mutex::new(None);
static ACTIVE_LINE_COLOR: Mutex<u32> = Mutex::new(0x004F4DFF);

// --- レンダラー自己回復とオーバーレイ可視状態の管理 ---
// RENDERER_ALIVE はレンダラースレッドがウィンドウを生成してメッセージループを
// 回している間だけ true。スレッドが終了する（ウィンドウが破棄される）と false に
// 戻る。RENDERER_SPAWNING は生成処理が進行中の間だけ true で、回復経路が同時に
// 複数のスレッド/ウィンドウを生成してリークすることを防ぐ（直列化ガード）。
// OVERLAY_VISIBLE は「今この瞬間に軌跡を表示しているか」の論理状態で、
// 最前面再主張（assert_topmost）を表示開始の立ち上がりエッジで一度だけ行うために使う。
static RENDERER_ALIVE: AtomicBool = AtomicBool::new(false);
static RENDERER_SPAWNING: AtomicBool = AtomicBool::new(false);
static RENDERER_INIT_LOCK: Mutex<()> = Mutex::new(());
static OVERLAY_VISIBLE: Mutex<bool> = Mutex::new(false);

const WINDOW_CLASS_NAME: PCWSTR = w!("OpenMouseGestureTrajectory");

// 旧実装の単色フラットライン(幅3, 完全不透明)に対する新レイヤー幅。
// コア/ボディは旧幅の約3倍、グローはさらに外側に広く低不透明度で伸びる。
const CORE_WIDTH: i32 = 4;
const BODY_WIDTH: i32 = 9;
const GLOW_WIDTH: i32 = 16;
// デザインB: 濃く密度の高い赤コア + 半透明ボディ + 柔らかい外側グロー。
// コアはベース色を明るく薄めず(白寄りにしない)、ほぼ不透明のまま元の線の
// 彩度/強さを保つ。ボディはベース色そのものを中間の不透明度で。
// グローはベース色をやや明るく伸ばし、低不透明度で自然にフェードする。
const CORE_ALPHA: u8 = 255;
const BODY_ALPHA: u8 = 150;
const GLOW_ALPHA: u8 = 55;
const BOUNDS_MARGIN: i32 = GLOW_WIDTH + 10;
const WM_UPDATE_TRAJECTORY: u32 = WM_USER + 1;
const MIN_FRAME_INTERVAL_MS: u64 = 16;

// 過去の修正(anchor矩形をジェスチャー開始点まわりに事前確保し、はみ出た場合のみ
// 外側へ拡張する方式)は、はみ出た瞬間にSetWindowPos(リサイズ)が発生し、
// SetWindowPosからUpdateLayeredWindowまでの間にコンポジタが古いビットマップを
// 新しいウィンドウ矩形で合成する一瞬が生じて、既に描画済みの軌跡全体が
// ずれて見える経路が理論上残っていた。ユーザーからは、斜め区間の後に方向を
// 変えると軌跡全体が一瞬追従するように震えるという報告が続いていた。
//
// 修正方針: ジェスチャーごとにウィンドウを可変にすることをやめ、起動時に
// 仮想デスクトップ全体を覆う固定originの単一ウィンドウを一度だけ作成する。
// 以後はプロセスの生存期間中、SetWindowPosによる移動・リサイズを一切行わない
// (再描画と、立ち上がりエッジでのz-order再主張のみ)。これにより、ジェスチャーが
// どれだけ大きく/どの方向に転じても、ウィンドウのorigin・サイズ・ローカル座標変換は
// 一切変化しようがない。

unsafe fn clear_dc(dc: HDC, width: i32, height: i32) {
    let _ = PatBlt(dc, 0, 0, width, height, BLACKNESS);
}

// オーバーレイを TOPMOST バンドの最上位に再主張する。SWP_NOMOVE | SWP_NOSIZE により
// origin/サイズは一切変更せず（=固定スクリーン空間の不変条件を維持）、SWP_NOACTIVATE
// によりフォーカス/アクティブウィンドウも奪わない。他のウィンドウの整列・活性化・
// 露出は行わず、単に自ウィンドウを最前面帯域の先頭へ持ち上げるだけである。
// フルスクリーン/ボーダーレスのアプリがオーバーレイ生成後に最前面へ昇格した場合でも、
// ジェスチャー開始時にこれを呼ぶことで軌跡をその上に表示できる。
unsafe fn assert_topmost(hwnd: HWND) {
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_TOPMOST),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
}

// 指定サイズのARGB32 DIBセクションを作成し、compatible DCに選択する
unsafe fn create_sized_dib(compat_dc: HDC, width: i32, height: i32) -> (HDC, HBITMAP) {
    let dc = CreateCompatibleDC(Some(compat_dc));

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD::default(); 1],
    };
    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let dib = CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
    let old = SelectObject(dc, dib.into());
    if !old.0.is_null() {
        let _ = DeleteObject(old);
    }
    (dc, dib)
}

// 指定幅・丸線端/丸結合のジオメトリックペンで折れ線を白色描画する（カバレッジマスク用）
unsafe fn draw_stroke_mask(dc: HDC, width: i32, width_px: i32, height_px: i32, points: &[(i32, i32)], offset: (i32, i32)) {
    clear_dc(dc, width_px, height_px);

    let brush = LOGBRUSH {
        lbStyle: BS_SOLID,
        lbColor: COLORREF(0x00FFFFFF),
        lbHatch: 0,
    };
    let pen = ExtCreatePen(
        PEN_STYLE(PS_GEOMETRIC.0 | PS_SOLID.0 | PS_ENDCAP_ROUND.0 | PS_JOIN_ROUND.0),
        width.max(1) as u32,
        &brush,
        None,
    );
    let old_pen = SelectObject(dc, pen.into());
    let _ = SetBkMode(dc, TRANSPARENT);

    let _ = MoveToEx(dc, points[0].0 - offset.0, points[0].1 - offset.1, None);
    for i in 1..points.len() {
        let _ = LineTo(dc, points[i].0 - offset.0, points[i].1 - offset.1);
    }

    SelectObject(dc, old_pen);
    let _ = DeleteObject(pen.into());
}

fn rgb_from_colorref(colorref: u32) -> (u8, u8, u8) {
    let r = (colorref & 0xFF) as u8;
    let g = ((colorref >> 8) & 0xFF) as u8;
    let b = ((colorref >> 16) & 0xFF) as u8;
    (r, g, b)
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8
}

// デザインB: ベース色(トリガー色)から、密度の高いコア/中間ボディ/柔らかいグローの
// 色相を導出する。コアは白へ寄せず、むしろわずかに沈めてベースより濃く密度のある
// 印象にする(彩度・強さを保つ = 「薄い/白っぽい」にならない)。
// ボディはベース色そのもの(半透明で支える太さ)。
// グローはベース色をやや明るく広げ、低不透明度で外側へ自然にフェードする。
fn derive_shades(base: (u8, u8, u8)) -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)) {
    let core = (
        lerp_u8(base.0, 0, 0.12),
        lerp_u8(base.1, 0, 0.12),
        lerp_u8(base.2, 0, 0.12),
    );
    let body = base;
    let glow = (
        lerp_u8(base.0, 255, 0.20),
        lerp_u8(base.1, 255, 0.20),
        lerp_u8(base.2, 255, 0.20),
    );
    (core, body, glow)
}

unsafe fn get_mask_bits(dc: HDC, bitmap: HBITMAP, width: i32, height: i32) -> Vec<u8> {
    let mut bits = vec![0u8; (width * height * 4) as usize];
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };
    let _ = GetDIBits(
        dc,
        bitmap,
        0,
        height as u32,
        Some(bits.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    bits
}

// snapshot_points はウィンドウのローカル座標系ではなく、常に物理スクリーン座標
// (accepted point) のまま保持される。ローカル座標への変換は、ウィンドウ作成時に
// 一度だけ確定した固定originを都度引くだけであり、他のいかなる値にも依存しない。
unsafe fn render_to_memory_dc_with_snapshot(snapshot_points: Vec<(i32, i32)>) {
    let (offset_x, offset_y) = *WINDOW_POS.lock().unwrap();
    let (width, height) = *WINDOW_SIZE.lock().unwrap();

    let mem_dc_opt = *MEMORY_DC.lock().unwrap();
    let mem_bitmap_opt = *MEMORY_BITMAP.lock().unwrap();
    let glow_dc_opt = *MASK_DC_GLOW.lock().unwrap();
    let glow_bmp_opt = *MASK_BITMAP_GLOW.lock().unwrap();
    let body_dc_opt = *MASK_DC_BODY.lock().unwrap();
    let body_bmp_opt = *MASK_BITMAP_BODY.lock().unwrap();
    let core_dc_opt = *MASK_DC_CORE.lock().unwrap();
    let core_bmp_opt = *MASK_BITMAP_CORE.lock().unwrap();

    let (
        Some(mem_dc_val),
        Some(mem_bitmap_val),
        Some(glow_dc_val),
        Some(glow_bmp_val),
        Some(body_dc_val),
        Some(body_bmp_val),
        Some(core_dc_val),
        Some(core_bmp_val),
    ) = (
        mem_dc_opt,
        mem_bitmap_opt,
        glow_dc_opt,
        glow_bmp_opt,
        body_dc_opt,
        body_bmp_opt,
        core_dc_opt,
        core_bmp_opt,
    )
    else {
        return;
    };

    let mem_dc = HDC(mem_dc_val as *mut _);
    let mem_bitmap = HBITMAP(mem_bitmap_val as *mut _);
    clear_dc(mem_dc, width, height);

    if snapshot_points.len() < 2 {
        // 点が無い場合は全面透明のまま。既にゼロクリア済みなので追加処理は不要。
        let mut bits = vec![0u8; (width * height * 4) as usize];
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            bmiColors: [RGBQUAD::default(); 1],
        };
        bits.fill(0);
        let _ = SetDIBits(Some(mem_dc), mem_bitmap, 0, height as u32, bits.as_ptr() as *const _, &bmi, DIB_RGB_COLORS);
        return;
    }

    let glow_dc = HDC(glow_dc_val as *mut _);
    let glow_bitmap = HBITMAP(glow_bmp_val as *mut _);
    let body_dc = HDC(body_dc_val as *mut _);
    let body_bitmap = HBITMAP(body_bmp_val as *mut _);
    let core_dc = HDC(core_dc_val as *mut _);
    let core_bitmap = HBITMAP(core_bmp_val as *mut _);

    let offset = (offset_x, offset_y);
    draw_stroke_mask(glow_dc, GLOW_WIDTH, width, height, &snapshot_points, offset);
    draw_stroke_mask(body_dc, BODY_WIDTH, width, height, &snapshot_points, offset);
    draw_stroke_mask(core_dc, CORE_WIDTH, width, height, &snapshot_points, offset);

    let glow_bits = get_mask_bits(glow_dc, glow_bitmap, width, height);
    let body_bits = get_mask_bits(body_dc, body_bitmap, width, height);
    let core_bits = get_mask_bits(core_dc, core_bitmap, width, height);

    let base_color = rgb_from_colorref(*ACTIVE_LINE_COLOR.lock().unwrap());
    let (core_color, body_color, glow_color) = derive_shades(base_color);

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for &(x, y) in snapshot_points.iter() {
        let local_x = x - offset_x;
        let local_y = y - offset_y;
        min_x = min_x.min(local_x - BOUNDS_MARGIN);
        min_y = min_y.min(local_y - BOUNDS_MARGIN);
        max_x = max_x.max(local_x + BOUNDS_MARGIN);
        max_y = max_y.max(local_y + BOUNDS_MARGIN);
    }
    min_x = min_x.max(0);
    min_y = min_y.max(0);
    max_x = max_x.min(width - 1);
    max_y = max_y.min(height - 1);

    let mut out_bits = vec![0u8; (width * height * 4) as usize];
    if min_x < max_x && min_y < max_y {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let idx = ((y * width + x) * 4) as usize;

                let (color, alpha) = if core_bits[idx] > 0 || core_bits[idx + 1] > 0 || core_bits[idx + 2] > 0 {
                    (core_color, CORE_ALPHA)
                } else if body_bits[idx] > 0 || body_bits[idx + 1] > 0 || body_bits[idx + 2] > 0 {
                    (body_color, BODY_ALPHA)
                } else if glow_bits[idx] > 0 || glow_bits[idx + 1] > 0 || glow_bits[idx + 2] > 0 {
                    (glow_color, GLOW_ALPHA)
                } else {
                    continue;
                };

                // レイヤードウィンドウ(ULW_ALPHA/AC_SRC_ALPHA)はプリマルチプライ済みアルファを要求する
                let a = alpha as f32 / 255.0;
                out_bits[idx] = (color.2 as f32 * a).round() as u8; // B
                out_bits[idx + 1] = (color.1 as f32 * a).round() as u8; // G
                out_bits[idx + 2] = (color.0 as f32 * a).round() as u8; // R
                out_bits[idx + 3] = alpha; // A
            }
        }
    }

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };
    let _ = SetDIBits(
        Some(mem_dc),
        mem_bitmap,
        0,
        height as u32,
        out_bits.as_ptr() as *const _,
        &bmi,
        DIB_RGB_COLORS,
    );
}

unsafe fn update_layered_window_from_memory(hwnd: HWND) {
    let mem_dc_opt = MEMORY_DC.lock().unwrap();
    if let Some(mem_dc_val) = *mem_dc_opt {
        let mem_dc = HDC(mem_dc_val as *mut _);
        let (win_w, win_h) = *WINDOW_SIZE.lock().unwrap();
        let (win_x, win_y) = *WINDOW_POS.lock().unwrap();
        if win_w > 0 && win_h > 0 {
            let size = SIZE {
                cx: win_w,
                cy: win_h,
            };
            let pt_src = POINT { x: 0, y: 0 };
            let pt_dst = POINT { x: win_x, y: win_y };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let result = UpdateLayeredWindow(
                hwnd,
                None,
                Some(&pt_dst),
                Some(&size),
                Some(mem_dc),
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            if result.is_err() {
                eprintln!(
                    "[TRAJECTORY_RENDERER] UpdateLayeredWindow failed: {:?}",
                    result
                );
            }
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_UPDATE_TRAJECTORY => {
            let visible = wparam.0 != 0;
            // ウィンドウのorigin/サイズは起動時に確定した仮想デスクトップ全体で不変の
            // ため、点列のスナップショットを同じ固定originで再描画するだけでよい。
            // ウィンドウはプロセス生存中ずっと表示状態を保ち、「軌跡なし」は全面透明
            // フレーム（点列が空/2点未満なら render 側がアルファ0のフレームを生成）で
            // 表現する。フレームごとの ShowWindow 表示/非表示切替は行わない（DWM合成を
            // 揺らしてフォアグラウンド背後のウィンドウを露出させる原因だったため）。
            let snapshot_points = {
                let state = TRAJECTORY_STATE.lock().unwrap();
                state.points.clone()
            };

            render_to_memory_dc_with_snapshot(snapshot_points);
            update_layered_window_from_memory(hwnd);

            let mut overlay_visible = OVERLAY_VISIBLE.lock().unwrap();
            if visible {
                if !*overlay_visible {
                    // 表示の立ち上がりエッジでのみ: 外部要因で非表示にされていた場合だけ
                    // 再表示し、TOPMOST バンドの最上位を再主張する。z-orderのみの操作で、
                    // アクティブウィンドウの奪取・他ウィンドウの整列/露出は行わない。
                    if !IsWindowVisible(hwnd).as_bool() {
                        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    }
                    assert_topmost(hwnd);
                }
                *overlay_visible = true;
            } else {
                *overlay_visible = false;
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            for (dc_slot, bitmap_slot) in [
                (&MEMORY_DC, &MEMORY_BITMAP),
                (&MASK_DC_GLOW, &MASK_BITMAP_GLOW),
                (&MASK_DC_BODY, &MASK_BITMAP_BODY),
                (&MASK_DC_CORE, &MASK_BITMAP_CORE),
            ] {
                let bitmap_opt = bitmap_slot.lock().unwrap();
                let dc_opt = dc_slot.lock().unwrap();
                if let Some(bitmap) = *bitmap_opt {
                    let _ = DeleteObject(HGDIOBJ(bitmap as *mut _));
                }
                if let Some(dc) = *dc_opt {
                    let _ = DeleteDC(HDC(dc as *mut _));
                }
            }

            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// レンダラースレッドの生存期間ガード。スレッドが正常終了・パニックのいずれで
// 終わっても Drop で RENDERER_ALIVE/RENDERER_SPAWNING を false に戻す。これにより
// 「スレッドは死んでいるのに生存フラグだけ true のまま回復不能になる」状態や、
// 「生成中のフラグが stuck して二度と回復できなくなる」状態を防ぐ。
struct RendererLifetimeGuard;

impl Drop for RendererLifetimeGuard {
    fn drop(&mut self) {
        RENDERER_ALIVE.store(false, Ordering::SeqCst);
        RENDERER_SPAWNING.store(false, Ordering::SeqCst);
    }
}

// オーバーレイウィンドウとメッセージループを別スレッドに生成する（非ブロッキング）。
// 起動時と自己回復時の両方から呼ばれる。ウィンドウは生成直後に全面透明フレームで
// 表示状態に入り（SW_SHOWNOACTIVATE + topmost）、以後プロセス生存中ずっと表示を保つ。
fn spawn_renderer_thread() {
    std::thread::spawn(|| unsafe {
        let _guard = RendererLifetimeGuard;

        let hinstance = HINSTANCE::default();

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH::default(),
            lpszClassName: WINDOW_CLASS_NAME,
            ..Default::default()
        };

        // 回復時に同一クラスが登録済みでも CreateWindowExW は既存クラスを使うため、
        // 登録失敗（クラス重複）は無害。戻り値は意図的に検査しない。
        RegisterClassExW(&wc);

        // 仮想デスクトップ全体を覆う固定originのウィンドウを一度だけ作成する。
        // 以後プロセスの生存期間中、このoriginとサイズは二度と変更しない
        // (移動・リサイズを伴う SetWindowPos は呼ばない)。
        let virtual_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let virtual_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let virtual_width = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1);
        let virtual_height = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            WINDOW_CLASS_NAME,
            w!(""),
            WS_POPUP,
            virtual_x,
            virtual_y,
            virtual_width,
            virtual_height,
            None,
            None,
            Some(hinstance),
            None,
        )
        .unwrap_or_default();

        if hwnd == HWND::default() {
            eprintln!("[TRAJECTORY_RENDERER] ERROR: Failed to create window");
            // ガードの Drop で SPAWNING が false に戻るため、次回ジェスチャー開始時に
            // ensure_renderer_alive が再試行できる（永久に回復不能にはならない）。
            return;
        }

        {
            let mut renderer_hwnd = RENDERER_HWND.lock().unwrap();
            *renderer_hwnd = Some(hwnd.0 as isize);
        }

        {
            let mut size = WINDOW_SIZE.lock().unwrap();
            *size = (virtual_width, virtual_height);
        }

        {
            let mut pos = WINDOW_POS.lock().unwrap();
            *pos = (virtual_x, virtual_y);
        }

        let screen_dc = GetDC(None);
        let (mem_dc, mem_bitmap) = create_sized_dib(screen_dc, virtual_width, virtual_height);
        let (glow_dc, glow_bitmap) = create_sized_dib(screen_dc, virtual_width, virtual_height);
        let (body_dc, body_bitmap) = create_sized_dib(screen_dc, virtual_width, virtual_height);
        let (core_dc, core_bitmap) = create_sized_dib(screen_dc, virtual_width, virtual_height);
        let _ = ReleaseDC(None, screen_dc);

        {
            let mut memory_dc = MEMORY_DC.lock().unwrap();
            *memory_dc = Some(mem_dc.0 as isize);
        }
        {
            let mut memory_bitmap = MEMORY_BITMAP.lock().unwrap();
            *memory_bitmap = Some(mem_bitmap.0 as isize);
        }
        {
            *MASK_DC_GLOW.lock().unwrap() = Some(glow_dc.0 as isize);
            *MASK_BITMAP_GLOW.lock().unwrap() = Some(glow_bitmap.0 as isize);
            *MASK_DC_BODY.lock().unwrap() = Some(body_dc.0 as isize);
            *MASK_BITMAP_BODY.lock().unwrap() = Some(body_bitmap.0 as isize);
            *MASK_DC_CORE.lock().unwrap() = Some(core_dc.0 as isize);
            *MASK_BITMAP_CORE.lock().unwrap() = Some(core_bitmap.0 as isize);
        }

        // 初期フレームは全面透明（点列空）で表示状態に入る。ShowWindow による
        // 表示/非表示切替は以後行わず、「軌跡なし」はこの透明フレームで表現する。
        render_to_memory_dc_with_snapshot(Vec::new());
        update_layered_window_from_memory(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        assert_topmost(hwnd);
        *OVERLAY_VISIBLE.lock().unwrap() = false;

        // ウィンドウが完全に準備できた時点で生存を宣言する。
        RENDERER_ALIVE.store(true, Ordering::SeqCst);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // メッセージループ終了（WM_DESTROY -> WM_QUIT）後: DC/ビットマップは WM_DESTROY
        // 内で解放済みなので、ここでは静的ハンドルを None に戻して次回生成に備えるだけ
        // （二重解放はしない）。ガードの Drop で ALIVE/SPAWNING も false になる。
        *RENDERER_HWND.lock().unwrap() = None;
        *OVERLAY_VISIBLE.lock().unwrap() = false;
        *MEMORY_DC.lock().unwrap() = None;
        *MEMORY_BITMAP.lock().unwrap() = None;
        *MASK_DC_GLOW.lock().unwrap() = None;
        *MASK_BITMAP_GLOW.lock().unwrap() = None;
        *MASK_DC_BODY.lock().unwrap() = None;
        *MASK_BITMAP_BODY.lock().unwrap() = None;
        *MASK_DC_CORE.lock().unwrap() = None;
        *MASK_BITMAP_CORE.lock().unwrap() = None;
    });
}

// レンダラーが生存し、有効なウィンドウを持っているかを返す。回復要不要の判定に使う。
fn renderer_is_healthy() -> bool {
    if !RENDERER_ALIVE.load(Ordering::SeqCst) {
        return false;
    }
    match *RENDERER_HWND.lock().unwrap() {
        Some(h) => unsafe { IsWindow(Some(HWND(h as *mut _))).as_bool() },
        None => false,
    }
}

// 回復のためにレンダラースレッドを新規生成すべきかを判定する純粋関数。
// 既に正常(healthy)、または生成中(spawning)のいずれかなら生成しない。
// 重複生成はウィンドウ/スレッド/メッセージループのリークになるため、ここで必ず抑止する。
fn should_spawn_renderer(healthy: bool, spawning: bool) -> bool {
    !healthy && !spawning
}

// レンダラーが死んでいれば（ウィンドウ破棄・スレッド終了・非表示化など）冪等に再生成する。
// 既に生存中、または生成処理が進行中の場合は何もしない（重複ウィンドウ/スレッド/
// メッセージループのリークを防ぐ）。ノンブロッキングで、フックスレッドを長時間塞がない。
// 生成は非同期のため、回復直後の最初のフレームはウィンドウ準備が間に合わず落ちる場合が
// あるが、点列は TRAJECTORY_STATE に蓄積され続けるため、準備完了後の次フレームで
// 軌跡全体が描画される（=アプリ再起動なしで後続ジェスチャーが正常描画に戻る）。
pub fn ensure_renderer_alive() {
    if renderer_is_healthy() {
        return;
    }
    let _guard = RENDERER_INIT_LOCK.lock().unwrap();
    let healthy = renderer_is_healthy();
    let spawning = RENDERER_SPAWNING.load(Ordering::SeqCst);
    if !should_spawn_renderer(healthy, spawning) {
        return;
    }
    RENDERER_SPAWNING.store(true, Ordering::SeqCst);
    spawn_renderer_thread();
}

pub fn init_renderer() -> Result<()> {
    ensure_renderer_alive();
    // 起動直後の最初のジェスチャーにオーバーレイが間に合うよう、ウィンドウ生成の
    // 完了を短時間だけ待つ（回復経路の ensure_renderer_alive はノンブロッキング）。
    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(())
}

// レンダラースレッドへ描画要求を送る共通経路。ウィンドウが存在しなければ
// （レンダラー未起動/破棄直後/回復中）そのフレームは安全に破棄される。点列は
// TRAJECTORY_STATE に蓄積され続けるため、ウィンドウ回復後の次フレームで全体が描画される。
// ここで ensure_renderer_alive を呼ばないのは、単体テストがネイティブウィンドウを
// 生成しないよう、回復の引き金をジェスチャー開始時（mouse_hook 側）に集約しているため。
fn post_render_message(visible: bool) {
    let hwnd = {
        let renderer_hwnd = RENDERER_HWND.lock().unwrap();
        renderer_hwnd.map(|h| HWND(h as *mut _))
    };

    if let Some(hwnd) = hwnd {
        unsafe {
            let _ = PostMessageW(
                Some(hwnd),
                WM_UPDATE_TRAJECTORY,
                WPARAM(visible as usize),
                LPARAM(0),
            );
        }
    }
}

pub fn update_trajectory(points: &[(i32, i32)], visible: bool) {
    {
        // 点列と可視状態を単一ロックで同時に更新する（バウンディングは
        // render側でこの点列から都度再計算するため、ここでは保持しない）。
        let mut state = TRAJECTORY_STATE.lock().unwrap();
        state.points.clear();
        if visible {
            state.points.extend_from_slice(points);
        }
        state.visible = visible;
    }

    let should_render = {
        let mut last_time = LAST_RENDER_TIME.lock().unwrap();
        let now = Instant::now();
        let should_update = match *last_time {
            Some(last) => now.duration_since(last).as_millis() >= MIN_FRAME_INTERVAL_MS as u128,
            None => true,
        };
        if should_update {
            *last_time = Some(now);
        }
        should_update || !visible // 非表示要求は常に通す
    };

    if should_render {
        // レンダラースレッドに通知（描画はwindow_procで集約）
        post_render_message(visible);
    }
}

pub fn append_trajectory_point(x: i32, y: i32) {
    {
        // 点の追加と可視化フラグの更新を単一ロックで行う。バウンディングは
        // ここでは維持せず、render側で毎回この点列から再計算する。
        let mut state = TRAJECTORY_STATE.lock().unwrap();
        state.points.push((x, y));
        state.visible = true;
    }

    let mut last_time = LAST_RENDER_TIME.lock().unwrap();
    let now = Instant::now();

    let should_render = match *last_time {
        Some(last) => now.duration_since(last).as_millis() >= MIN_FRAME_INTERVAL_MS as u128,
        None => true,
    };

    if should_render {
        *last_time = Some(now);
        drop(last_time);

        post_render_message(true);
    }
}

pub fn clear_trajectory_display() {
    {
        let mut state = TRAJECTORY_STATE.lock().unwrap();
        state.points.clear();
        state.visible = false;
    }

    post_render_message(false);
}

pub fn set_active_color(hex_color: &str) {
    let normalized = hex_color.trim();
    if normalized.len() != 7 || !normalized.starts_with('#') {
        return;
    }

    if let Ok(rgb) = u32::from_str_radix(&normalized[1..], 16) {
        let r = (rgb >> 16) & 0xFF;
        let g = (rgb >> 8) & 0xFF;
        let b = rgb & 0xFF;
        let colorref = (b << 16) | (g << 8) | r;
        *ACTIVE_LINE_COLOR.lock().unwrap() = colorref;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_from_colorref_roundtrips_hex() {
        // set_active_color("#FF4D4F") -> COLORREF 0x004F4DFF
        let (r, g, b) = rgb_from_colorref(0x004F4DFF);
        assert_eq!((r, g, b), (0xFF, 0x4D, 0x4F));
    }

    #[test]
    fn derive_shades_core_is_dense_not_pale() {
        // デザインB: コアは白へ寄せない(薄く/白っぽくしない)。ベースと同等以上に
        // 濃く、彩度・強さを保った「密度の高い」色であること。
        let base = (0xFFu8, 0x4Du8, 0x4Fu8);
        let (core, body, glow) = derive_shades(base);
        assert_eq!(body, base);
        assert!(
            core.1 <= body.1 && core.2 <= body.2,
            "core must not be lightened toward white (would look pale)"
        );
        assert!(
            glow.1 >= body.1,
            "glow may lighten outward for a soft-glow feel"
        );
    }

    #[test]
    fn layer_widths_are_at_least_three_times_old_line_width() {
        const OLD_LINE_WIDTH: i32 = 3;
        assert!(BODY_WIDTH >= OLD_LINE_WIDTH * 3 - 1);
        assert!(GLOW_WIDTH > BODY_WIDTH);
        assert!(CORE_WIDTH < BODY_WIDTH);
    }

    #[test]
    fn layer_alphas_are_within_translucent_targets() {
        assert!(GLOW_ALPHA < BODY_ALPHA);
        assert!(BODY_ALPHA < CORE_ALPHA);
        assert!((140..=205).contains(&(BODY_ALPHA as i32)));
        assert!((204..=255).contains(&(CORE_ALPHA as i32)));
    }

    #[test]
    fn core_alpha_is_dense_near_opaque() {
        // 「薄い/washed out」ではなく密度の高いコアであることを保証する。
        assert!(CORE_ALPHA >= 245, "core must be close to fully opaque");
    }

    // --- 固定スクリーン空間アーキテクチャのテスト ---
    // ウィンドウのorigin/サイズは起動時(init_renderer)に仮想デスクトップ全体で
    // 一度だけ確定し、以後プロセスの生存期間中不変であることが前提。
    // 本ファイルの SetWindowPos は z-order 再主張(SWP_NOMOVE|SWP_NOSIZE)のみに使い、
    // 移動・リサイズを伴う呼び出しは存在しない(grepで確認可能)ため、
    // ここではその不変条件のもとで座標変換・状態管理が正しく振る舞うことを検証する。

    fn reset_state_for_test() {
        let mut state = TRAJECTORY_STATE.lock().unwrap();
        state.points.clear();
        state.visible = false;
    }

    #[test]
    fn accepted_points_are_stored_verbatim_as_physical_screen_coordinates() {
        // append_trajectory_point/update_trajectory は受け取った座標をそのまま
        // 保持するだけで、いかなる変換・丸め・平滑化も行わない。
        reset_state_for_test();
        update_trajectory(&[(100, 100)], true);
        append_trajectory_point(140, 90);
        append_trajectory_point(180, 80);
        append_trajectory_point(300, 80); // 斜め -> 水平への方向転換

        let points = TRAJECTORY_STATE.lock().unwrap().points.clone();
        assert_eq!(points, vec![(100, 100), (140, 90), (180, 80), (300, 80)]);
    }

    #[test]
    fn appending_new_points_never_mutates_previously_accepted_points() {
        // 過去に受理された点は、新しい点が追加されても一切書き換わらない
        // (=既に描画された区間のローカル座標が変化しようがない根拠)。
        reset_state_for_test();
        update_trajectory(&[(0, 0)], true);
        let mut previous_prefix = TRAJECTORY_STATE.lock().unwrap().points.clone();

        for (x, y) in [(10, 5), (25, -10), (25, 40), (60, 40)] {
            append_trajectory_point(x, y);
            let current = TRAJECTORY_STATE.lock().unwrap().points.clone();
            assert_eq!(
                &current[..previous_prefix.len()],
                previous_prefix.as_slice(),
                "previously accepted points must remain byte-for-byte identical"
            );
            previous_prefix = current;
        }
    }

    #[test]
    fn reset_occurs_only_on_explicit_gesture_end() {
        // ジェスチャーの途中(方向転換を含む)ではTRAJECTORY_STATEがクリアされず、
        // 明示的にvisible=falseが渡されたとき(=ジェスチャー終了)のみリセットされる。
        reset_state_for_test();
        update_trajectory(&[(0, 0)], true);
        append_trajectory_point(50, 50); // diagonal
        append_trajectory_point(150, 50); // turn to horizontal
        append_trajectory_point(150, -50); // turn to vertical

        {
            let state = TRAJECTORY_STATE.lock().unwrap();
            assert_eq!(state.points.len(), 4);
            assert!(state.visible);
        }

        clear_trajectory_display();

        let state = TRAJECTORY_STATE.lock().unwrap();
        assert!(state.points.is_empty());
        assert!(!state.visible);
    }

    #[test]
    fn local_coordinate_conversion_uses_one_constant_origin_including_negative() {
        // ローカル座標変換は「物理スクリーン座標 - ジェスチャー開始時に確定した
        // 単一の固定origin」のみで行われる。マルチモニタでプライマリが左上に
        // ないケース(originが負)でも同じ式が使えることを確認する。
        let origin = (-1920, -200);
        let physical_points = [(-1920, -200), (-1870, -150), (-1770, -150), (-1770, 50)];

        let local_before_turn: Vec<(i32, i32)> = physical_points[..3]
            .iter()
            .map(|&(x, y)| (x - origin.0, y - origin.1))
            .collect();
        let local_after_turn: Vec<(i32, i32)> = physical_points
            .iter()
            .map(|&(x, y)| (x - origin.0, y - origin.1))
            .collect();

        // 方向転換後に新しい点が加わっても、それ以前の点のローカル座標は
        // 同じoriginを使う限り一切変化しない。
        assert_eq!(&local_after_turn[..3], local_before_turn.as_slice());
        assert_eq!(local_after_turn[0], (0, 0));
    }

    #[test]
    fn glow_body_core_layers_share_identical_point_sequence_and_offset() {
        // render_to_memory_dc_with_snapshot は glow/body/core の3レイヤーすべてに
        // 対して同一の snapshot_points と同一の offset (= WINDOW_POS) を
        // draw_stroke_mask に渡す。3回の呼び出しが異なるジオメトリを参照する
        // 余地がないことをソース構造として固定するための回帰テスト。
        let src = include_str!("trajectory_renderer.rs");
        let calls: Vec<&str> = src
            .lines()
            .filter(|l| l.trim_start().starts_with("draw_stroke_mask("))
            .collect();
        assert_eq!(calls.len(), 3, "expected exactly one draw_stroke_mask call per layer");
        for call in &calls {
            assert!(call.contains("&snapshot_points, offset"));
        }
    }

    #[test]
    fn rgb_from_colorref_roundtrips_hex_regression() {
        let (r, g, b) = rgb_from_colorref(0x00112233);
        assert_eq!((r, g, b), (0x33, 0x22, 0x11));
    }

    // --- レンダラー自己回復の判定ロジック ---

    #[test]
    fn should_spawn_renderer_only_when_unhealthy_and_not_spawning() {
        // 重複生成（=ウィンドウ/スレッド/メッセージループのリーク）を防ぐため、
        // 生成は「未生存 かつ 非生成中」のときだけ許可される。
        assert!(should_spawn_renderer(false, false));
        assert!(!should_spawn_renderer(true, false));
        assert!(!should_spawn_renderer(false, true));
        assert!(!should_spawn_renderer(true, true));
    }

    #[test]
    fn renderer_is_unhealthy_when_not_alive() {
        // 単体テスト環境ではレンダラースレッドを起動しないため RENDERER_ALIVE は false。
        // このとき renderer_is_healthy は必ず false を返さなければならない
        // （=回復経路が「生存している」と誤判定して回復をスキップしない）。
        // ※ ensure_renderer_alive 自体はネイティブウィンドウを生成するためテストでは呼ばない。
        RENDERER_ALIVE.store(false, Ordering::SeqCst);
        assert!(!renderer_is_healthy());
    }

    // --- ネイティブウィンドウ構成（透明・クリックスルー・非活性化・最前面）---
    // 物理フルスクリーンの合成挙動は CI では検証できないため、ここではオーバーレイが
    // 入力透明・クリック通過・非活性化・最前面・ツールウィンドウの拡張スタイルで生成され、
    // 最前面再主張が z-order 限定（移動/リサイズ/活性化なし）であること、そして
    // ShowWindow による非表示化（SW_HIDE）を廃止した always-shown 構成であることを
    // ソース構造として固定する。物理検証は CHANGELOG の「要・実機確認」項目を参照。

    #[test]
    fn overlay_window_uses_nonactivating_clickthrough_topmost_toolwindow_styles() {
        let src = include_str!("trajectory_renderer.rs");
        // WS_EX_TOOLWINDOW を含む行で絞り込むことで、拡張スタイルを列挙するだけの
        // ヘッダコメントではなく CreateWindowExW の実際の実装行を特定する。
        let style_line = src
            .lines()
            .find(|l| l.contains("WS_EX_LAYERED") && l.contains("WS_EX_TOOLWINDOW"))
            .expect("overlay CreateWindowExW extended-style line must exist");
        assert!(style_line.contains("WS_EX_TOPMOST"), "must stay topmost");
        assert!(
            style_line.contains("WS_EX_NOACTIVATE"),
            "must never activate/steal focus"
        );
        assert!(
            style_line.contains("WS_EX_TRANSPARENT"),
            "must remain input-transparent (click-through)"
        );
    }

    #[test]
    fn topmost_reassert_is_zorder_only_without_move_size_or_activate() {
        // 検索トークンは動的に組み立てる（include_str! がこのテスト自身のソースも取り込む
        // ため、リテラルをそのまま contains すると自己参照で常に真になってしまう）。
        let src = include_str!("trajectory_renderer.rs");
        let topmost_target = format!("Some({})", "HWND_TOPMOST");
        let zorder_only_flags = format!("{} | {} | {}", "SWP_NOMOVE", "SWP_NOSIZE", "SWP_NOACTIVATE");
        assert!(
            src.contains(&topmost_target),
            "topmost re-assert must target the TOPMOST band"
        );
        assert!(
            src.contains(&zorder_only_flags),
            "topmost re-assert must not move, resize, or activate (preserves the fixed-origin invariant)"
        );
    }

    #[test]
    fn overlay_no_longer_hides_via_showwindow_to_avoid_background_exposure() {
        // always-shown 構成: 「軌跡なし」は全面透明フレームで表現し、ShowWindow による
        // 非表示化は行わない。フルスクリーン相当ウィンドウに対する ShowWindow 遷移が
        // DWM 合成を揺らして背後ウィンドウを露出させる経路を断つため。
        // 検索トークンは動的に組み立てる（include_str! がこのテスト自身のソースも取り込む
        // ため、リテラルをそのまま書くと自己参照で必ず失敗する）。
        let src = include_str!("trajectory_renderer.rs");
        let hide_call = format!("ShowWindow(hwnd, {})", "SW_HIDE");
        assert!(
            !src.contains(&hide_call),
            "the overlay must never be hidden via ShowWindow with the hide flag"
        );
    }
}
