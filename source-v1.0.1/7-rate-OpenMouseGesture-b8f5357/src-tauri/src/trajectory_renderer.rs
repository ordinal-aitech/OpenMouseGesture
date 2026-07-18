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
static WINDOW_OFFSET: Mutex<(i32, i32)> = Mutex::new((0, 0));
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

// 点列から一貫した式でバウンディング矩形を導出する。
// window_procでのウィンドウ配置・DIBクリッピング・描画オフセットが常にこの1つの
// 式・1つのスナップショットから計算されることを保証し、ジオメトリのずれを防ぐ。
fn compute_bounds(points: &[(i32, i32)]) -> Option<RECT> {
    if points.is_empty() {
        return None;
    }

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for &(x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    Some(RECT {
        left: min_x - BOUNDS_MARGIN,
        top: min_y - BOUNDS_MARGIN,
        right: max_x + BOUNDS_MARGIN,
        bottom: max_y + BOUNDS_MARGIN,
    })
}

unsafe fn clear_dc(dc: HDC, width: i32, height: i32) {
    let _ = PatBlt(dc, 0, 0, width, height, BLACKNESS);
}

unsafe fn clear_memory_dc() {
    let mem_dc_opt = MEMORY_DC.lock().unwrap();
    if let Some(mem_dc_val) = *mem_dc_opt {
        let mem_dc = HDC(mem_dc_val as *mut _);
        let (width, height) = *WINDOW_SIZE.lock().unwrap();
        clear_dc(mem_dc, width, height);
    }
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

unsafe fn realloc_dib_slot(
    dc_slot: &Mutex<Option<isize>>,
    bitmap_slot: &Mutex<Option<isize>>,
    compat_dc: HDC,
    width: i32,
    height: i32,
) {
    {
        let dc_opt = *dc_slot.lock().unwrap();
        let bitmap_opt = *bitmap_slot.lock().unwrap();
        if let Some(bitmap_val) = bitmap_opt {
            let _ = DeleteObject(HGDIOBJ(bitmap_val as *mut _));
        }
        if let Some(dc_val) = dc_opt {
            let _ = DeleteDC(HDC(dc_val as *mut _));
        }
    }

    let (dc, bitmap) = create_sized_dib(compat_dc, width, height);
    *dc_slot.lock().unwrap() = Some(dc.0 as isize);
    *bitmap_slot.lock().unwrap() = Some(bitmap.0 as isize);
}

// 軌跡のバウンディングにウィンドウとDIBを合わせる（スナップショット版）
unsafe fn ensure_window_matches_bounds_with_snapshot(hwnd: HWND, snapshot_bounds: Option<RECT>) {
    if let Some(r) = snapshot_bounds {
        let left = r.left;
        let top = r.top;
        let w = (r.right - r.left).max(1);
        let h = (r.bottom - r.top).max(1);

        // 現在のウィンドウ情報
        let (cur_w, cur_h) = *WINDOW_SIZE.lock().unwrap();
        let (cur_x, cur_y) = *WINDOW_POS.lock().unwrap();

        let need_realloc = w != cur_w || h != cur_h;
        let need_move = left != cur_x || top != cur_y;

        if need_realloc {
            let mem_dc_val_opt = *MEMORY_DC.lock().unwrap();
            if let Some(mem_dc_val) = mem_dc_val_opt {
                let mem_dc = HDC(mem_dc_val as *mut _);
                realloc_dib_slot(&MEMORY_DC, &MEMORY_BITMAP, mem_dc, w, h);
                realloc_dib_slot(&MASK_DC_GLOW, &MASK_BITMAP_GLOW, mem_dc, w, h);
                realloc_dib_slot(&MASK_DC_BODY, &MASK_BITMAP_BODY, mem_dc, w, h);
                realloc_dib_slot(&MASK_DC_CORE, &MASK_BITMAP_CORE, mem_dc, w, h);

                {
                    let mut size = WINDOW_SIZE.lock().unwrap();
                    *size = (w, h);
                }

                clear_memory_dc();
            }
        }

        if need_move || need_realloc {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                left,
                top,
                w,
                h,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOREDRAW,
            );
            {
                let mut pos = WINDOW_POS.lock().unwrap();
                *pos = (left, top);
            }
            {
                let mut off = WINDOW_OFFSET.lock().unwrap();
                // ローカル座標変換用に、オフセット=ウィンドウ左上
                *off = (left, top);
            }
        }
    }
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

unsafe fn render_to_memory_dc_with_snapshot(
    snapshot_bounds: Option<RECT>,
    snapshot_points: Vec<(i32, i32)>,
) {
    let (offset_x, offset_y) = if let Some(rect) = snapshot_bounds {
        (rect.left, rect.top)
    } else {
        (0, 0)
    };
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
            if visible {
                // 単一Mutexから点列を1回のロックでスナップショットし、バウンディングは
                // 必ずこのスナップショットから再計算する。点列とバウンディングが
                // 別々のタイミングで観測されることがなくなり、ウィンドウ位置・
                // クリッピング・描画オフセットが常に同じジオメトリを参照する。
                let snapshot_points = {
                    let state = TRAJECTORY_STATE.lock().unwrap();
                    state.points.clone()
                };
                let snapshot_bounds = compute_bounds(&snapshot_points);

                ensure_window_matches_bounds_with_snapshot(hwnd, snapshot_bounds.clone());
                render_to_memory_dc_with_snapshot(snapshot_bounds, snapshot_points);
                update_layered_window_from_memory(hwnd);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            } else {
                clear_memory_dc();
                update_layered_window_from_memory(hwnd);
                let _ = ShowWindow(hwnd, SW_HIDE);
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

pub fn init_renderer() -> Result<()> {
    std::thread::spawn(|| unsafe {
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

        RegisterClassExW(&wc);

        let _virtual_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let _virtual_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let _virtual_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let _virtual_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            WINDOW_CLASS_NAME,
            w!(""),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(hinstance),
            None,
        )
        .unwrap_or_default();

        if hwnd == HWND::default() {
            eprintln!("[TRAJECTORY_RENDERER] ERROR: Failed to create window");
            return;
        }

        {
            let mut renderer_hwnd = RENDERER_HWND.lock().unwrap();
            *renderer_hwnd = Some(hwnd.0 as isize);
        }

        {
            let mut offset = WINDOW_OFFSET.lock().unwrap();
            *offset = (0, 0);
        }

        {
            let mut size = WINDOW_SIZE.lock().unwrap();
            *size = (1, 1);
        }

        {
            let mut pos = WINDOW_POS.lock().unwrap();
            *pos = (0, 0);
        }

        let screen_dc = GetDC(None);
        let (mem_dc, mem_bitmap) = create_sized_dib(screen_dc, 1, 1);
        let (glow_dc, glow_bitmap) = create_sized_dib(screen_dc, 1, 1);
        let (body_dc, body_bitmap) = create_sized_dib(screen_dc, 1, 1);
        let (core_dc, core_bitmap) = create_sized_dib(screen_dc, 1, 1);
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

        clear_memory_dc();
        update_layered_window_from_memory(hwnd);
        let _ = ShowWindow(hwnd, SW_HIDE);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(())
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
        let hwnd = {
            let renderer_hwnd = RENDERER_HWND.lock().unwrap();
            renderer_hwnd.map(|h| HWND(h as *mut _))
        };

        if let Some(hwnd) = hwnd {
            // レンダラースレッドに通知（描画はwindow_procで集約）
            unsafe {
                let _ = PostMessageW(
                    Some(hwnd),
                    WM_UPDATE_TRAJECTORY,
                    WPARAM(if visible { 1 } else { 0 }),
                    LPARAM(0),
                );
            }
        }
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

        let hwnd = {
            let renderer_hwnd = RENDERER_HWND.lock().unwrap();
            renderer_hwnd.map(|h| HWND(h as *mut _))
        };

        if let Some(hwnd) = hwnd {
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_UPDATE_TRAJECTORY, WPARAM(1), LPARAM(0));
            }
        }
    }
}

pub fn clear_trajectory_display() {
    {
        let mut state = TRAJECTORY_STATE.lock().unwrap();
        state.points.clear();
        state.visible = false;
    }

    let hwnd = {
        let renderer_hwnd = RENDERER_HWND.lock().unwrap();
        renderer_hwnd.map(|h| HWND(h as *mut _))
    };

    if let Some(hwnd) = hwnd {
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_UPDATE_TRAJECTORY, WPARAM(0), LPARAM(0));
        }
    }
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

    // --- ジオメトリ安定性(ジッター修正)のテスト ---
    // compute_bounds は常に同一の点列スナップショットから同じ式で
    // バウンディングを導出する。これにより、window/DIBの配置と描画オフセットが
    // フレーム間で不整合になる(=軌跡が震える)ことを防ぐ。

    #[test]
    fn compute_bounds_is_none_for_empty_points() {
        assert!(compute_bounds(&[]).is_none());
    }

    #[test]
    fn compute_bounds_is_deterministic_for_same_points() {
        let points = vec![(10, 20), (50, 5), (30, 80), (-10, 40)];
        let a = compute_bounds(&points).unwrap();
        let b = compute_bounds(&points).unwrap();
        assert_eq!((a.left, a.top, a.right, a.bottom), (b.left, b.top, b.right, b.bottom));
    }

    #[test]
    fn compute_bounds_is_invariant_to_point_insertion_order() {
        // 到着順に関わらず、同じ点集合なら同じバウンディングになることを保証する。
        // これは append_trajectory_point による逐次追加と、
        // update_trajectory によるバルク更新が同じ結果になることの根拠。
        let ordered = vec![(-10, 40), (10, 20), (30, 80), (50, 5)];
        let shuffled = vec![(50, 5), (10, 20), (-10, 40), (30, 80)];
        let a = compute_bounds(&ordered).unwrap();
        let b = compute_bounds(&shuffled).unwrap();
        assert_eq!((a.left, a.top, a.right, a.bottom), (b.left, b.top, b.right, b.bottom));
    }

    #[test]
    fn compute_bounds_matches_margin_formula() {
        let points = vec![(100, 200)];
        let bounds = compute_bounds(&points).unwrap();
        assert_eq!(bounds.left, 100 - BOUNDS_MARGIN);
        assert_eq!(bounds.top, 200 - BOUNDS_MARGIN);
        assert_eq!(bounds.right, 100 + BOUNDS_MARGIN);
        assert_eq!(bounds.bottom, 200 + BOUNDS_MARGIN);
    }

    #[test]
    fn compute_bounds_grows_monotonically_as_points_are_appended() {
        // 軌跡が伸びるにつれてバウンディングは単調に拡大するのみで、
        // 縮小して既存の点がクリップされることがないことを保証する
        // (これも「既に描画された経路が視覚的に固定されている」ための前提)。
        let mut points = vec![(0, 0)];
        let mut prev = compute_bounds(&points).unwrap();
        for p in [(5, -5), (-20, 10), (30, 30), (-5, -40)] {
            points.push(p);
            let next = compute_bounds(&points).unwrap();
            assert!(next.left <= prev.left);
            assert!(next.top <= prev.top);
            assert!(next.right >= prev.right);
            assert!(next.bottom >= prev.bottom);
            prev = next;
        }
    }
}
