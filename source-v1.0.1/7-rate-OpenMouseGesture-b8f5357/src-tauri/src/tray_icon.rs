/*
概要: システムトレイアイコンの有効/無効状態に応じたPNGバイト列選択と、
アイコン画像の余白（透明領域）検証に使う純粋な解析関数を扱うモジュール。
実際にトレイへ反映する処理（`TrayIcon::set_icon`呼び出し）はlib.rs側で行う。
入出力:
  - 入力: 有効/無効の状態フラグ、PNGバイト列
  - 出力: 埋め込み済みPNGバイト列、アルファ有効領域の境界比率、赤色優勢ピクセル比率
具体例:
  - `icon_bytes_for_state(false)` は無効状態用（グレー基調+赤バツ）のPNGを返す
  - `content_coverage_ratio(ENABLED_ICON_BYTES)` はキャンバスに対する図柄占有率(0.0-1.0)を返す
*/
use image::GenericImageView;

pub const ENABLED_ICON_BYTES: &[u8] = include_bytes!("../icons/tray_enabled_128.png");
pub const DISABLED_ICON_BYTES: &[u8] = include_bytes!("../icons/tray_disabled_128.png");

/// アルファ値がこの値以上のピクセルを「描画済み」とみなす閾値。
#[allow(dead_code)]
const ALPHA_CONTENT_THRESHOLD: u8 = 8;

pub fn icon_bytes_for_state(enabled: bool) -> &'static [u8] {
    if enabled {
        ENABLED_ICON_BYTES
    } else {
        DISABLED_ICON_BYTES
    }
}

/// PNGバイト列を読み込み、不透明とみなせるピクセルの外接矩形を
/// (left, top, right, bottom)（両端含む）で返す。全透明の場合は None。
#[allow(dead_code)]
pub fn alpha_content_bounds(bytes: &[u8]) -> Option<(u32, u32, u32, u32)> {
    let img = image::load_from_memory(bytes).ok()?;
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();

    let mut left = width;
    let mut right = 0i64;
    let mut top = height;
    let mut bottom = 0i64;
    let mut found = false;

    for (x, y, pixel) in rgba.enumerate_pixels() {
        if pixel[3] >= ALPHA_CONTENT_THRESHOLD {
            found = true;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x as i64);
            bottom = bottom.max(y as i64);
        }
    }

    if !found {
        return None;
    }

    Some((left, top, right as u32, bottom as u32))
}

/// 図柄がキャンバスに対してどれだけの割合を占めているか（余白がどれだけ
/// 少ないか）を、外接矩形の長辺 / キャンバス辺 で近似して返す。
#[allow(dead_code)]
pub fn content_coverage_ratio(bytes: &[u8]) -> Option<f32> {
    let img = image::load_from_memory(bytes).ok()?;
    let (width, height) = img.dimensions();
    let (left, top, right, bottom) = alpha_content_bounds(bytes)?;

    let content_w = (right - left + 1) as f32;
    let content_h = (bottom - top + 1) as f32;
    let canvas = width.max(height) as f32;

    Some(content_w.max(content_h) / canvas)
}

/// 赤色が支配的なピクセル（無効状態バッジの赤バツ判定に使用）の
/// 不透明ピクセル全体に対する比率を返す。判定は R が高く、かつ G と B が
/// 互いに近い値（= オレンジ系ではなく無彩に近い赤）であることを要求する。
/// アプリ本体のオレンジ色モチーフ（R高・G中・B低）を誤検出しないため。
#[allow(dead_code)]
pub fn red_dominant_pixel_ratio(bytes: &[u8]) -> Option<f32> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();

    let mut opaque_count = 0u32;
    let mut red_count = 0u32;

    for pixel in rgba.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < ALPHA_CONTENT_THRESHOLD {
            continue;
        }
        opaque_count += 1;
        let r = r as i32;
        let g = g as i32;
        let b = b as i32;
        if r > 150 && r - g > 60 && r - b > 60 && (g - b).abs() < 40 {
            red_count += 1;
        }
    }

    if opaque_count == 0 {
        return None;
    }

    Some(red_count as f32 / opaque_count as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_bytes_for_state_selects_enabled_and_disabled_assets() {
        assert_eq!(icon_bytes_for_state(true), ENABLED_ICON_BYTES);
        assert_eq!(icon_bytes_for_state(false), DISABLED_ICON_BYTES);
    }

    #[test]
    fn enabled_and_disabled_icons_are_distinct_assets() {
        assert_ne!(ENABLED_ICON_BYTES, DISABLED_ICON_BYTES);
    }

    #[test]
    fn enabled_icon_has_minimal_transparent_padding() {
        // Previously the shipped tray asset only filled ~86% of the canvas.
        // The regenerated asset must occupy nearly the full safe canvas.
        let ratio = content_coverage_ratio(ENABLED_ICON_BYTES).expect("icon should decode");
        assert!(
            ratio >= 0.90,
            "expected tightly-cropped tray icon, got coverage ratio {}",
            ratio
        );
    }

    #[test]
    fn disabled_icon_has_minimal_transparent_padding() {
        let ratio = content_coverage_ratio(DISABLED_ICON_BYTES).expect("icon should decode");
        assert!(
            ratio >= 0.90,
            "expected tightly-cropped disabled tray icon, got coverage ratio {}",
            ratio
        );
    }

    #[test]
    fn disabled_icon_contains_a_prominent_red_x_overlay() {
        let ratio =
            red_dominant_pixel_ratio(DISABLED_ICON_BYTES).expect("icon should decode");
        assert!(
            ratio >= 0.10,
            "expected a prominent red overlay on the disabled icon, got red ratio {}",
            ratio
        );
    }

    #[test]
    fn enabled_icon_has_negligible_red_dominant_pixels() {
        // Sanity check that the red-X detector is meaningful: the normal
        // (enabled) icon's orange motif must not itself trip the same
        // "prominent red" threshold used to detect the disabled badge.
        let ratio = red_dominant_pixel_ratio(ENABLED_ICON_BYTES).expect("icon should decode");
        assert!(
            ratio < 0.10,
            "enabled icon unexpectedly looks like the disabled red-X icon: {}",
            ratio
        );
    }

    #[test]
    fn alpha_content_bounds_are_within_canvas() {
        let img = image::load_from_memory(ENABLED_ICON_BYTES).expect("icon should decode");
        let (width, height) = img.dimensions();
        let (left, top, right, bottom) =
            alpha_content_bounds(ENABLED_ICON_BYTES).expect("icon should have content");
        assert!(right < width);
        assert!(bottom < height);
        assert!(left <= right);
        assert!(top <= bottom);
    }
}
