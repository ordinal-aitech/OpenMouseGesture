use std::sync::Mutex;

use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*, Win32::UI::WindowsAndMessaging::*,
};

static LABEL_HWND: Mutex<Option<isize>> = Mutex::new(None);
static MEMORY_DC: Mutex<Option<isize>> = Mutex::new(None);
static MEMORY_BITMAP: Mutex<Option<isize>> = Mutex::new(None);
static LABEL_TEXT: Mutex<Option<(String, Option<String>)>> = Mutex::new(None);
static OVERLAY_READY: Mutex<bool> = Mutex::new(false);

const WINDOW_CLASS_NAME: PCWSTR = w!("OpenMouseGestureActionLabel");
const LABEL_WIDTH: i32 = 420;
const LABEL_HEIGHT: i32 = 108;
const LABEL_MARGIN_BOTTOM: i32 = 88;
const LABEL_RADIUS: i32 = 18;
const LABEL_BG_COLOR: COLORREF = COLORREF(0x00338CFF);
const LABEL_TEXT_COLOR: COLORREF = COLORREF(0x00FFFFFF);

unsafe fn clear_memory_dc() {
    let mem_dc_opt = MEMORY_DC.lock().unwrap();
    if let Some(mem_dc_val) = *mem_dc_opt {
        let mem_dc = HDC(mem_dc_val as *mut _);
        let _ = PatBlt(mem_dc, 0, 0, LABEL_WIDTH, LABEL_HEIGHT, BLACKNESS);
    }
}

unsafe fn fix_alpha_channel() {
    let mem_dc_opt = MEMORY_DC.lock().unwrap();
    let mem_bitmap_opt = MEMORY_BITMAP.lock().unwrap();

    let Some(mem_dc_val) = *mem_dc_opt else {
        return;
    };
    let Some(bitmap_val) = *mem_bitmap_opt else {
        return;
    };

    let mem_dc = HDC(mem_dc_val as *mut _);
    let bitmap = HBITMAP(bitmap_val as *mut _);

    let mut bits = vec![0u8; (LABEL_WIDTH * LABEL_HEIGHT * 4) as usize];
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: LABEL_WIDTH,
            biHeight: -LABEL_HEIGHT,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };

    let _ = GetDIBits(
        mem_dc,
        bitmap,
        0,
        LABEL_HEIGHT as u32,
        Some(bits.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );

    for px in bits.chunks_exact_mut(4) {
        let visible = px[0] > 0 || px[1] > 0 || px[2] > 0;
        px[3] = if visible { 255 } else { 0 };
    }

    let _ = SetDIBits(
        Some(mem_dc),
        bitmap,
        0,
        LABEL_HEIGHT as u32,
        bits.as_ptr() as *const _,
        &bmi,
        DIB_RGB_COLORS,
    );
}

unsafe fn render_label_to_memory_dc(primary: &str, secondary: Option<&str>) {
    clear_memory_dc();

    let mem_dc_opt = MEMORY_DC.lock().unwrap();
    let Some(mem_dc_val) = *mem_dc_opt else {
        return;
    };
    let mem_dc = HDC(mem_dc_val as *mut _);

    let brush = CreateSolidBrush(LABEL_BG_COLOR);
    let old_brush = SelectObject(mem_dc, brush.into());
    let old_pen = SelectObject(mem_dc, GetStockObject(NULL_PEN));
    let _ = RoundRect(
        mem_dc,
        0,
        0,
        LABEL_WIDTH,
        LABEL_HEIGHT,
        LABEL_RADIUS,
        LABEL_RADIUS,
    );
    let _ = SelectObject(mem_dc, old_pen);
    let _ = SelectObject(mem_dc, old_brush);
    let _ = DeleteObject(brush.into());

    let _ = SetBkMode(mem_dc, TRANSPARENT);
    let _ = SetTextColor(mem_dc, LABEL_TEXT_COLOR);

    let font_primary = CreateFontW(
        20,
        0,
        0,
        0,
        FW_BOLD.0 as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH.0 as u32,
        w!("Segoe UI"),
    );
    let font_secondary = CreateFontW(
        16,
        0,
        0,
        0,
        FW_MEDIUM.0 as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH.0 as u32,
        w!("Segoe UI"),
    );

    let old_font_primary = SelectObject(mem_dc, font_primary.into());
    let mut primary_wide: Vec<u16> = primary.encode_utf16().collect();
    let mut primary_rect = RECT {
        left: 18,
        top: if secondary.is_some() { 10 } else { 20 },
        right: LABEL_WIDTH - 18,
        bottom: if secondary.is_some() {
            40
        } else {
            LABEL_HEIGHT - 16
        },
    };
    let _ = DrawTextW(
        mem_dc,
        &mut primary_wide,
        &mut primary_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
    );
    let _ = SelectObject(mem_dc, old_font_primary);
    let _ = DeleteObject(font_primary.into());

    if let Some(secondary_text) = secondary {
        let old_font_secondary = SelectObject(mem_dc, font_secondary.into());
        let mut secondary_wide: Vec<u16> = secondary_text.encode_utf16().collect();
        let mut secondary_rect = RECT {
            left: 18,
            top: 38,
            right: LABEL_WIDTH - 18,
            bottom: LABEL_HEIGHT - 12,
        };
        let _ = DrawTextW(
            mem_dc,
            &mut secondary_wide,
            &mut secondary_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        let _ = SelectObject(mem_dc, old_font_secondary);
    }
    let _ = DeleteObject(font_secondary.into());

    fix_alpha_channel();
}

unsafe fn update_layered_window_from_memory(hwnd: HWND) {
    let mem_dc_opt = MEMORY_DC.lock().unwrap();
    let Some(mem_dc_val) = *mem_dc_opt else {
        return;
    };
    let mem_dc = HDC(mem_dc_val as *mut _);

    let virtual_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let virtual_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
    let virtual_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let virtual_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

    let x = virtual_x + (virtual_width - LABEL_WIDTH) / 2;
    let y = virtual_y + virtual_height - LABEL_HEIGHT - LABEL_MARGIN_BOTTOM;

    let size = SIZE {
        cx: LABEL_WIDTH,
        cy: LABEL_HEIGHT,
    };
    let pt_src = POINT { x: 0, y: 0 };
    let pt_dst = POINT { x, y };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    let _ = UpdateLayeredWindow(
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
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            let mem_dc_opt = MEMORY_DC.lock().unwrap();
            let mem_bitmap_opt = MEMORY_BITMAP.lock().unwrap();

            if let Some(bitmap) = *mem_bitmap_opt {
                let _ = DeleteObject(HGDIOBJ(bitmap as *mut _));
            }
            if let Some(dc) = *mem_dc_opt {
                let _ = DeleteDC(HDC(dc as *mut _));
            }

            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn apply_label_state() {
    let hwnd = {
        let label_hwnd = LABEL_HWND.lock().unwrap();
        label_hwnd.map(|h| HWND(h as *mut _))
    };

    let Some(hwnd) = hwnd else {
        return;
    };

    unsafe {
        let label = LABEL_TEXT.lock().unwrap().clone();
        if let Some((primary, secondary)) = label {
            eprintln!(
                "[ACTION_LABEL] show primary='{}' secondary='{}'",
                primary,
                secondary.as_deref().unwrap_or("")
            );
            render_label_to_memory_dc(&primary, secondary.as_deref());
            update_layered_window_from_memory(hwnd);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        } else {
            eprintln!("[ACTION_LABEL] hide");
            clear_memory_dc();
            update_layered_window_from_memory(hwnd);
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn init_overlay() -> Result<()> {
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

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            WINDOW_CLASS_NAME,
            w!(""),
            WS_POPUP,
            0,
            0,
            LABEL_WIDTH,
            LABEL_HEIGHT,
            None,
            None,
            Some(hinstance),
            None,
        )
        .unwrap_or_default();

        if hwnd == HWND::default() {
            return;
        }

        let screen_dc = GetDC(None);
        let mem_dc = CreateCompatibleDC(Some(screen_dc));

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: LABEL_WIDTH,
                biHeight: -LABEL_HEIGHT,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            bmiColors: [RGBQUAD::default(); 1],
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(Some(mem_dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();

        let _ = SelectObject(mem_dc, dib.into());
        let _ = ReleaseDC(None, screen_dc);

        {
            let mut memory_dc = MEMORY_DC.lock().unwrap();
            *memory_dc = Some(mem_dc.0 as isize);
        }
        {
            let mut memory_bitmap = MEMORY_BITMAP.lock().unwrap();
            *memory_bitmap = Some(dib.0 as isize);
        }
        {
            let mut label_hwnd = LABEL_HWND.lock().unwrap();
            *label_hwnd = Some(hwnd.0 as isize);
        }
        {
            let mut ready = OVERLAY_READY.lock().unwrap();
            *ready = true;
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

    for _ in 0..50 {
        if *OVERLAY_READY.lock().unwrap() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Ok(())
}

pub fn update_action_label(primary: Option<&str>, secondary: Option<&str>) {
    {
        let mut label = LABEL_TEXT.lock().unwrap();
        *label = primary.map(|p| (p.to_string(), secondary.map(|s| s.to_string())));
    }
    if *OVERLAY_READY.lock().unwrap() {
        apply_label_state();
    }
}

pub fn clear_action_label() {
    update_action_label(None, None);
}
