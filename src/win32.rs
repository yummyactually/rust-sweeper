//! Raw Win32/GDI bindings and the native window/event loop.
//!
//! This module deliberately depends only on `std`: it contains the complete
//! FFI surface required by the game, owns the window procedure, and provides
//! a small cached GDI drawing canvas for `app`.

#![allow(non_snake_case, dead_code, clippy::upper_case_acronyms)]

use std::collections::HashMap;
use std::ffi::c_void;

pub type Color = u32;
pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    (b as u32) << 16 | (g as u32) << 8 | r as u32
}

type BOOL = i32;
type DWORD = u32;
type UINT = u32;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type ATOM = u16;
type LPCWSTR = *const u16;
type HINSTANCE = *mut c_void;
type HMODULE = *mut c_void;
type HWND = *mut c_void;
type HDC = *mut c_void;
type HGDIOBJ = *mut c_void;
type HBRUSH = *mut c_void;
type HPEN = *mut c_void;
type HFONT = *mut c_void;
type HBITMAP = *mut c_void;
type HICON = *mut c_void;
type HCURSOR = *mut c_void;
type HMENU = *mut c_void;
type TIMERPROC = Option<unsafe extern "system" fn(HWND, UINT, usize, DWORD)>;

const WM_DESTROY: UINT = 0x0002;
const WM_MOUSEMOVE: UINT = 0x0200;
const WM_LBUTTONDOWN: UINT = 0x0201;
const WM_LBUTTONUP: UINT = 0x0202;
const WM_RBUTTONDOWN: UINT = 0x0204;
const WM_RBUTTONUP: UINT = 0x0205;
const WM_MBUTTONDOWN: UINT = 0x0207;
const WM_MBUTTONUP: UINT = 0x0208;
const WM_KEYDOWN: UINT = 0x0100;
const WM_PAINT: UINT = 0x000F;
const WM_TIMER: UINT = 0x0113;
const WM_KILLFOCUS: UINT = 0x0008;
const WM_MOUSELEAVE: UINT = 0x02A3;
const WM_SETCURSOR: UINT = 0x0020;
const WM_CONTEXTMENU: UINT = 0x007B;
const WM_ERASEBKGND: UINT = 0x0014;
const WM_COMMAND: UINT = 0x0111;
const WM_SETICON: UINT = 0x0080;
const WM_SIZE: UINT = 0x0005;

const WS_OVERLAPPED: DWORD = 0x00000000;
const WS_CAPTION: DWORD = 0x00C00000;
const WS_SYSMENU: DWORD = 0x00080000;
const WS_MINIMIZEBOX: DWORD = 0x00020000;
const SW_SHOW: i32 = 5;
const HTCLIENT: u32 = 1;
const GWLP_USERDATA: i32 = -21;
const GWL_STYLE: i32 = -16;
const TIMER_ID: usize = 1;
const TME_LEAVE: DWORD = 0x00000002;
const SPI_GETWORKAREA: UINT = 0x0030;
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;
const IDC_ARROW: usize = 32512;
const IDC_HAND: usize = 32649;
const ICON_SMALL: usize = 0;
const ICON_BIG: usize = 1;
const MB_OK: UINT = 0x00000000;
const MB_ICONERROR: UINT = 0x00000010;
const SRCCOPY: DWORD = 0x00CC0020;
const TRANSPARENT: i32 = 1;
const DEFAULT_CHARSET: DWORD = 1;
const CLEARTYPE_QUALITY: DWORD = 5;
const CLASS_ALREADY_EXISTS: u32 = 1410;
const COLOR_BTNFACE: i32 = 15;
const MB_ICONINFORMATION: UINT = 0x00000040;
const SWP_NOZORDER: UINT = 0x0004;
const SWP_NOACTIVATE: UINT = 0x0010;
const MF_STRING: UINT = 0x00000000;
const MF_POPUP: UINT = 0x00000010;
const MF_SEPARATOR: UINT = 0x00000800;
const MF_CHECKED: UINT = 0x00000008;
const MF_UNCHECKED: UINT = 0x00000000;
const MF_BYCOMMAND: UINT = 0x00000000;

/// Native menu command identifiers shared with `app`.
pub const CMD_NEW_GAME: u16 = 1001;
pub const CMD_BEGINNER: u16 = 1002;
pub const CMD_INTERMEDIATE: u16 = 1003;
pub const CMD_EXPERT: u16 = 1004;
pub const CMD_MARKS: u16 = 1005;
pub const CMD_EXIT: u16 = 1006;
pub const CMD_ABOUT: u16 = 1007;

pub const DT_LEFT: u32 = 0x0000;
pub const DT_CENTER: u32 = 0x0001;
pub const DT_RIGHT: u32 = 0x0002;
pub const DT_VCENTER: u32 = 0x0004;
pub const DT_WORDBREAK: u32 = 0x0010;
pub const DT_SINGLELINE: u32 = 0x0020;
pub const DT_NOPREFIX: u32 = 0x0800;
pub const DT_END_ELLIPSIS: u32 = 0x8000;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn from_xywh(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self::new(x, y, x.saturating_add(width), y.saturating_add(height))
    }

    pub const fn width(self) -> i32 {
        self.right - self.left
    }

    pub const fn height(self) -> i32 {
        self.bottom - self.top
    }

    pub const fn center_x(self) -> i32 {
        self.left + self.width() / 2
    }

    pub const fn center_y(self) -> i32 {
        self.top + self.height() / 2
    }

    pub fn inset(self, amount: i32) -> Self {
        Self::new(
            self.left + amount,
            self.top + amount,
            self.right - amount,
            self.bottom - amount,
        )
    }

    pub fn inset_hv(self, horizontal: i32, vertical: i32) -> Self {
        Self::new(
            self.left + horizontal,
            self.top + vertical,
            self.right - horizontal,
            self.bottom - vertical,
        )
    }

    pub fn expand(self, amount: i32) -> Self {
        self.inset(-amount)
    }

    pub fn contains(self, point: Point) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }

    pub fn centered_in(self, area: Rect) -> Self {
        let x = area.center_x() - self.width() / 2;
        let y = area.center_y() - self.height() / 2;
        Self::from_xywh(x, y, self.width(), self.height())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WinRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl From<Rect> for WinRect {
    fn from(value: Rect) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WinPoint {
    x: i32,
    y: i32,
}

#[repr(C)]
struct PaintStruct {
    hdc: HDC,
    f_erase: BOOL,
    rc_paint: WinRect,
    f_restore: BOOL,
    f_inc_update: BOOL,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
struct Message {
    hwnd: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
    time: DWORD,
    point: WinPoint,
}

#[repr(C)]
struct TrackMouseEventData {
    cb_size: DWORD,
    dw_flags: DWORD,
    hwnd_track: HWND,
    dw_hover_time: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct BitmapInfoHeader {
    size: DWORD,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: DWORD,
    size_image: DWORD,
    x_pels_per_meter: i32,
    y_pels_per_meter: i32,
    colors_used: DWORD,
    colors_important: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [DWORD; 1],
}

#[repr(C)]
struct IconInfo {
    is_icon: BOOL,
    hotspot_x: DWORD,
    hotspot_y: DWORD,
    mask: HBITMAP,
    color: HBITMAP,
}

type WndProc = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

#[repr(C)]
struct WndClassW {
    style: UINT,
    wnd_proc: WndProc,
    class_extra: i32,
    window_extra: i32,
    instance: HINSTANCE,
    icon: HICON,
    cursor: HCURSOR,
    background: HBRUSH,
    menu_name: LPCWSTR,
    class_name: LPCWSTR,
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassW(wnd_class: *const WndClassW) -> ATOM;
    fn CreateWindowExW(
        ex_style: DWORD,
        class_name: LPCWSTR,
        window_name: LPCWSTR,
        style: DWORD,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    fn DefWindowProcW(hwnd: HWND, message: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn DestroyWindow(hwnd: HWND) -> BOOL;
    fn ShowWindow(hwnd: HWND, command: i32) -> BOOL;
    fn UpdateWindow(hwnd: HWND) -> BOOL;
    fn GetMessageW(message: *mut Message, hwnd: HWND, min: UINT, max: UINT) -> BOOL;
    fn TranslateMessage(message: *const Message) -> BOOL;
    fn DispatchMessageW(message: *const Message) -> LRESULT;
    fn PostQuitMessage(exit_code: i32);
    fn BeginPaint(hwnd: HWND, paint: *mut PaintStruct) -> HDC;
    fn EndPaint(hwnd: HWND, paint: *const PaintStruct) -> BOOL;
    fn GetClientRect(hwnd: HWND, rect: *mut WinRect) -> BOOL;
    fn InvalidateRect(hwnd: HWND, rect: *const WinRect, erase: BOOL) -> BOOL;
    fn SetTimer(hwnd: HWND, id: usize, elapse: UINT, callback: TIMERPROC) -> usize;
    fn KillTimer(hwnd: HWND, id: usize) -> BOOL;
    fn LoadCursorW(instance: HINSTANCE, cursor_name: LPCWSTR) -> HCURSOR;
    fn LoadIconW(instance: HINSTANCE, icon_name: LPCWSTR) -> HICON;
    fn SetCursor(cursor: HCURSOR) -> HCURSOR;
    fn GetCursorPos(point: *mut WinPoint) -> BOOL;
    fn ScreenToClient(hwnd: HWND, point: *mut WinPoint) -> BOOL;
    fn TrackMouseEvent(event: *mut TrackMouseEventData) -> BOOL;
    fn GetModuleHandleW(module_name: LPCWSTR) -> HMODULE;
    fn AdjustWindowRectEx(rect: *mut WinRect, style: DWORD, menu: BOOL, ex_style: DWORD) -> BOOL;
    fn SetProcessDPIAware() -> BOOL;
    fn GetSystemMetrics(index: i32) -> i32;
    fn SystemParametersInfoW(action: UINT, param: UINT, data: *mut c_void, flags: UINT) -> BOOL;
    fn MessageBoxW(hwnd: HWND, text: LPCWSTR, caption: LPCWSTR, kind: UINT) -> i32;
    fn SetWindowLongPtrW(hwnd: HWND, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
    fn SetFocus(hwnd: HWND) -> HWND;
    fn GetLastError() -> u32;
    fn IsWindow(hwnd: HWND) -> BOOL;
    fn SendMessageW(hwnd: HWND, message: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn GetActiveWindow() -> HWND;
    fn UnregisterClassW(class_name: LPCWSTR, instance: HINSTANCE) -> BOOL;
    fn CreateMenu() -> HMENU;
    fn GetSubMenu(menu: HMENU, position: i32) -> HMENU;
    fn CreatePopupMenu() -> HMENU;
    fn AppendMenuW(menu: HMENU, flags: UINT, item: usize, text: LPCWSTR) -> BOOL;
    fn CheckMenuRadioItem(
        menu: HMENU,
        first: UINT,
        last: UINT,
        checked_item: UINT,
        flags: UINT,
    ) -> BOOL;
    fn CheckMenuItem(menu: HMENU, item: UINT, check: UINT) -> DWORD;
    fn SetMenu(hwnd: HWND, menu: HMENU) -> BOOL;
    fn DrawMenuBar(hwnd: HWND) -> BOOL;
    fn DestroyMenu(menu: HMENU) -> BOOL;
    fn GetSysColorBrush(index: i32) -> HBRUSH;
    fn SetWindowPos(
        hwnd: HWND,
        insert_after: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: UINT,
    ) -> BOOL;
    fn SetCapture(hwnd: HWND) -> HWND;
    fn ReleaseCapture() -> BOOL;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(dc: HDC) -> HDC;
    fn CreateCompatibleBitmap(dc: HDC, width: i32, height: i32) -> HBITMAP;
    fn CreateBitmap(
        width: i32,
        height: i32,
        planes: UINT,
        bit_count: UINT,
        bits: *const c_void,
    ) -> HBITMAP;
    fn CreateDIBSection(
        dc: HDC,
        info: *const BitmapInfo,
        usage: UINT,
        bits: *mut *mut c_void,
        section: *mut c_void,
        offset: DWORD,
    ) -> HBITMAP;
    fn CreateIconIndirect(info: *const IconInfo) -> HICON;
    fn DestroyIcon(icon: HICON) -> BOOL;
    fn SelectObject(dc: HDC, object: HGDIOBJ) -> HGDIOBJ;
    fn DeleteObject(object: HGDIOBJ) -> BOOL;
    fn DeleteDC(dc: HDC) -> BOOL;
    fn BitBlt(
        dest: HDC,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        source: HDC,
        source_x: i32,
        source_y: i32,
        rop: DWORD,
    ) -> BOOL;
    fn FillRect(dc: HDC, rect: *const WinRect, brush: HBRUSH) -> i32;
    fn CreateSolidBrush(color: Color) -> HBRUSH;
    fn CreatePen(style: i32, width: i32, color: Color) -> HPEN;
    fn MoveToEx(dc: HDC, x: i32, y: i32, old: *mut WinPoint) -> BOOL;
    fn LineTo(dc: HDC, x: i32, y: i32) -> BOOL;
    fn Polyline(dc: HDC, points: *const WinPoint, count: i32) -> BOOL;
    fn Polygon(dc: HDC, points: *const WinPoint, count: i32) -> BOOL;
    fn Ellipse(dc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> BOOL;
    fn RoundRect(
        dc: HDC,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        ellipse_width: i32,
        ellipse_height: i32,
    ) -> BOOL;
    fn SetTextColor(dc: HDC, color: Color) -> Color;
    fn SetBkMode(dc: HDC, mode: i32) -> i32;
    fn DrawTextW(dc: HDC, text: LPCWSTR, count: i32, rect: *mut WinRect, format: UINT) -> i32;
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: DWORD,
        underline: DWORD,
        strike_out: DWORD,
        char_set: DWORD,
        out_precision: DWORD,
        clip_precision: DWORD,
        quality: DWORD,
        pitch_and_family: DWORD,
        face_name: LPCWSTR,
    ) -> HFONT;
}

/// A durable GDI font. The handle is released by `Drop`.
pub struct Font(HFONT);

impl Font {
    pub fn new(face: &str, pixel_height: i32, weight: i32, italic: bool) -> Result<Self, String> {
        let face = to_wide(face);
        let height = pixel_height.abs().max(1);
        let handle = unsafe {
            CreateFontW(
                -height,
                0,
                0,
                0,
                weight,
                italic as DWORD,
                0,
                0,
                DEFAULT_CHARSET,
                0,
                0,
                CLEARTYPE_QUALITY,
                0,
                face.as_ptr(),
            )
        };
        if handle.is_null() {
            Err(format!(
                "Could not create the interface font (Win32 error {}).",
                unsafe { GetLastError() }
            ))
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                DeleteObject(self.0);
            }
            self.0 = std::ptr::null_mut();
        }
    }
}

struct OwnedIcon(HICON);

impl OwnedIcon {
    fn create(size: i32) -> Result<Self, String> {
        let size = size.max(8);
        let pixels = scaled_mine_icon(size);
        let width = size;
        let height = size;
        let stride_bytes = (size as usize).div_ceil(16) * 2;
        let mask_bytes = vec![0u8; stride_bytes * size as usize];
        let info = BitmapInfo {
            header: BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as DWORD,
                width,
                height: -height,
                planes: 1,
                bit_count: 32,
                compression: 0,
                size_image: 0,
                x_pels_per_meter: 0,
                y_pels_per_meter: 0,
                colors_used: 0,
                colors_important: 0,
            },
            colors: [0],
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let color = unsafe {
            CreateDIBSection(
                std::ptr::null_mut(),
                &info,
                0,
                &mut bits,
                std::ptr::null_mut(),
                0,
            )
        };
        if color.is_null() || bits.is_null() {
            if !color.is_null() {
                unsafe {
                    DeleteObject(color);
                }
            }
            return Err(last_error("Could not allocate the Minesweeper icon pixels"));
        }
        let target = unsafe { std::slice::from_raw_parts_mut(bits as *mut u32, pixels.len()) };
        target.copy_from_slice(&pixels);

        let mask =
            unsafe { CreateBitmap(width, height, 1, 1, mask_bytes.as_ptr() as *const c_void) };
        if mask.is_null() {
            unsafe {
                DeleteObject(color);
            }
            return Err(last_error("Could not create the Minesweeper icon mask"));
        }

        let icon_info = IconInfo {
            is_icon: 1,
            hotspot_x: 0,
            hotspot_y: 0,
            mask,
            color,
        };
        let icon = unsafe { CreateIconIndirect(&icon_info) };
        unsafe {
            DeleteObject(mask);
            DeleteObject(color);
        }
        if icon.is_null() {
            Err(last_error("Could not create the Minesweeper icon"))
        } else {
            Ok(Self(icon))
        }
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                DestroyIcon(self.0);
            }
            self.0 = std::ptr::null_mut();
        }
    }
}

fn scaled_mine_icon(size: i32) -> Vec<u32> {
    let base = mine_icon_pixels_16();
    if size == 16 {
        return base;
    }
    let target = size.max(1) as usize;
    let mut result = vec![0u32; target * target];
    for y in 0..target {
        for x in 0..target {
            let source_x = (x as f32 * 16.0 / target as f32) as usize;
            let source_y = (y as f32 * 16.0 / target as f32) as usize;
            let source_x = source_x.min(15);
            let source_y = source_y.min(15);
            if let Some(pixel) = base.get(source_y * 16 + source_x) {
                if let Some(slot) = result.get_mut(y * target + x) {
                    *slot = *pixel;
                }
            }
        }
    }
    result
}

fn mine_icon_pixels_16() -> Vec<u32> {
    const TILE: u32 = 0xffc0_c0c0;
    const WHITE: u32 = 0xffff_ffff;
    const DARK: u32 = 0xff80_8080;
    const BLACK: u32 = 0xff10_1010;

    let mut pixels = vec![0u32; 16 * 16];
    for y in 1..15 {
        for x in 1..15 {
            set_icon_pixel(&mut pixels, 16, x, y, TILE);
        }
    }
    for x in 1..15 {
        set_icon_pixel(&mut pixels, 16, x, 1, WHITE);
        set_icon_pixel(&mut pixels, 16, x, 14, DARK);
    }
    for y in 1..15 {
        set_icon_pixel(&mut pixels, 16, 1, y, WHITE);
        set_icon_pixel(&mut pixels, 16, 14, y, DARK);
    }

    let spikes = [
        (8, 2),
        (13, 8),
        (8, 13),
        (3, 8),
        (4, 4),
        (12, 4),
        (12, 12),
        (4, 12),
    ];
    for (x, y) in spikes {
        draw_icon_line(&mut pixels, 16, 8, 8, x, y, BLACK);
    }
    fill_icon_circle(&mut pixels, 16, 8, 8, 3.6, BLACK);
    set_icon_pixel(&mut pixels, 16, 6, 6, WHITE);
    set_icon_pixel(&mut pixels, 16, 6, 7, WHITE);
    set_icon_pixel(&mut pixels, 16, 7, 6, WHITE);
    pixels
}

fn set_icon_pixel(pixels: &mut [u32], size: i32, x: i32, y: i32, color: u32) {
    if x < 0 || y < 0 || x >= size || y >= size {
        return;
    }
    let index = y as usize * size as usize + x as usize;
    if let Some(slot) = pixels.get_mut(index) {
        *slot = color;
    }
}

fn draw_icon_line(
    pixels: &mut [u32],
    size: i32,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        set_icon_pixel(pixels, size, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let doubled = error.saturating_mul(2);
        if doubled >= dy {
            error += dy;
            x0 += sx;
        }
        if doubled <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn fill_icon_circle(
    pixels: &mut [u32],
    size: i32,
    center_x: i32,
    center_y: i32,
    radius: f32,
    color: u32,
) {
    let radius_squared = radius * radius;
    for y in 0..size {
        for x in 0..size {
            let dx = (x - center_x) as f32;
            let dy = (y - center_y) as f32;
            if dx * dx + dy * dy <= radius_squared {
                set_icon_pixel(pixels, size, x, y, color);
            }
        }
    }
}

/// A small cached GDI painter used for one double-buffered frame.
pub struct Canvas {
    dc: HDC,
    brushes: HashMap<Color, HBRUSH>,
    pens: HashMap<(Color, i32), HPEN>,
}

impl Canvas {
    fn new(dc: HDC) -> Self {
        Self {
            dc,
            brushes: HashMap::new(),
            pens: HashMap::new(),
        }
    }

    fn brush(&mut self, color: Color) -> HBRUSH {
        if let Some(handle) = self.brushes.get(&color) {
            return *handle;
        }
        let handle = unsafe { CreateSolidBrush(color) };
        if !handle.is_null() {
            self.brushes.insert(color, handle);
        }
        handle
    }

    fn pen(&mut self, color: Color, width: i32) -> HPEN {
        let width = width.max(1);
        if let Some(handle) = self.pens.get(&(color, width)) {
            return *handle;
        }
        let handle = unsafe { CreatePen(0, width, color) };
        if !handle.is_null() {
            self.pens.insert((color, width), handle);
        }
        handle
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        let brush = self.brush(color);
        if brush.is_null() {
            return;
        }
        let raw = WinRect::from(rect);
        unsafe {
            FillRect(self.dc, &raw, brush);
        }
    }

    /// A subtle banded gradient. Broad solid bands keep it crisp and cheap.
    pub fn fill_gradient(&mut self, rect: Rect, top: Color, bottom: Color, steps: i32) {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        let steps = steps.max(1);
        let height = rect.height();
        let top_r = (top & 0xff) as i32;
        let top_g = ((top >> 8) & 0xff) as i32;
        let top_b = ((top >> 16) & 0xff) as i32;
        let bottom_r = (bottom & 0xff) as i32;
        let bottom_g = ((bottom >> 8) & 0xff) as i32;
        let bottom_b = ((bottom >> 16) & 0xff) as i32;

        for index in 0..steps {
            let y0 = rect.top + height * index / steps;
            let y1 = rect.top + height * (index + 1) / steps;
            if y1 <= y0 {
                continue;
            }
            let t = index as f32 / steps as f32;
            let r = (top_r as f32 + (bottom_r - top_r) as f32 * t) as i32;
            let g = (top_g as f32 + (bottom_g - top_g) as f32 * t) as i32;
            let b = (top_b as f32 + (bottom_b - top_b) as f32 * t) as i32;
            let color = rgb(
                r.clamp(0, 255) as u8,
                g.clamp(0, 255) as u8,
                b.clamp(0, 255) as u8,
            );
            self.fill_rect(Rect::new(rect.left, y0, rect.right, y1), color);
        }
    }

    pub fn rounded_rect(
        &mut self,
        rect: Rect,
        radius: i32,
        fill: Color,
        border: Color,
        border_width: i32,
    ) {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        let brush = self.brush(fill);
        let pen = self.pen(border, border_width.max(1));
        if brush.is_null() || pen.is_null() {
            return;
        }
        let diameter = (radius.max(0) * 2).max(1);
        unsafe {
            let old_brush = SelectObject(self.dc, brush);
            let old_pen = SelectObject(self.dc, pen);
            RoundRect(
                self.dc,
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                diameter,
                diameter,
            );
            if !old_pen.is_null() {
                SelectObject(self.dc, old_pen);
            }
            if !old_brush.is_null() {
                SelectObject(self.dc, old_brush);
            }
        }
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Color, width: i32) {
        let width = width.max(1);
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        self.line(rect.left, rect.top, rect.right - 1, rect.top, color, width);
        self.line(
            rect.right - 1,
            rect.top,
            rect.right - 1,
            rect.bottom - 1,
            color,
            width,
        );
        self.line(
            rect.right - 1,
            rect.bottom - 1,
            rect.left,
            rect.bottom - 1,
            color,
            width,
        );
        self.line(
            rect.left,
            rect.bottom - 1,
            rect.left,
            rect.top,
            color,
            width,
        );
    }

    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color, width: i32) {
        let pen = self.pen(color, width);
        if pen.is_null() {
            return;
        }
        unsafe {
            let old_pen = SelectObject(self.dc, pen);
            MoveToEx(self.dc, x1, y1, std::ptr::null_mut());
            LineTo(self.dc, x2, y2);
            if !old_pen.is_null() {
                SelectObject(self.dc, old_pen);
            }
        }
    }

    pub fn polyline(&mut self, points: &[Point], color: Color, width: i32) {
        if points.len() < 2 {
            return;
        }
        let count = match i32::try_from(points.len()) {
            Ok(count) => count,
            Err(_) => return,
        };
        let pen = self.pen(color, width);
        if pen.is_null() {
            return;
        }
        unsafe {
            let old_pen = SelectObject(self.dc, pen);
            Polyline(self.dc, points.as_ptr().cast(), count);
            if !old_pen.is_null() {
                SelectObject(self.dc, old_pen);
            }
        }
    }

    pub fn polygon(&mut self, points: &[Point], fill: Color, border: Color, width: i32) {
        if points.len() < 3 {
            return;
        }
        let count = match i32::try_from(points.len()) {
            Ok(count) => count,
            Err(_) => return,
        };
        let brush = self.brush(fill);
        let pen = self.pen(border, width);
        if brush.is_null() || pen.is_null() {
            return;
        }
        unsafe {
            let old_brush = SelectObject(self.dc, brush);
            let old_pen = SelectObject(self.dc, pen);
            Polygon(self.dc, points.as_ptr().cast(), count);
            if !old_pen.is_null() {
                SelectObject(self.dc, old_pen);
            }
            if !old_brush.is_null() {
                SelectObject(self.dc, old_brush);
            }
        }
    }

    pub fn ellipse(&mut self, rect: Rect, fill: Color, border: Color, border_width: i32) {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        let brush = self.brush(fill);
        let pen = self.pen(border, border_width.max(1));
        if brush.is_null() || pen.is_null() {
            return;
        }
        unsafe {
            let old_brush = SelectObject(self.dc, brush);
            let old_pen = SelectObject(self.dc, pen);
            Ellipse(self.dc, rect.left, rect.top, rect.right, rect.bottom);
            if !old_pen.is_null() {
                SelectObject(self.dc, old_pen);
            }
            if !old_brush.is_null() {
                SelectObject(self.dc, old_brush);
            }
        }
    }

    pub fn text(&mut self, text: &str, rect: Rect, font: &Font, color: Color, format: u32) {
        if font.0.is_null() || rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        let encoded: Vec<u16> = text.encode_utf16().collect();
        let count = match i32::try_from(encoded.len()) {
            Ok(count) => count,
            Err(_) => return,
        };
        let mut raw = WinRect::from(rect);
        unsafe {
            let old_font = SelectObject(self.dc, font.0);
            let old_color = SetTextColor(self.dc, color);
            let old_mode = SetBkMode(self.dc, TRANSPARENT);
            DrawTextW(self.dc, encoded.as_ptr(), count, &mut raw, format);
            let _ = SetBkMode(self.dc, old_mode);
            let _ = SetTextColor(self.dc, old_color);
            if !old_font.is_null() {
                SelectObject(self.dc, old_font);
            }
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            for (_, handle) in self.brushes.drain() {
                if !handle.is_null() {
                    DeleteObject(handle);
                }
            }
            for (_, handle) in self.pens.drain() {
                if !handle.is_null() {
                    DeleteObject(handle);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    MouseMove { x: i32, y: i32 },
    MouseLeave,
    LeftDown { x: i32, y: i32 },
    LeftUp { x: i32, y: i32 },
    RightDown { x: i32, y: i32 },
    MiddleDown { x: i32, y: i32 },
    Timer,
    KeyDown(u32),
    Command(u16),
    FocusLost,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EventResult {
    pub redraw: bool,
    pub quit: bool,
}

impl EventResult {
    pub const NONE: Self = Self {
        redraw: false,
        quit: false,
    };
    pub const REDRAW: Self = Self {
        redraw: true,
        quit: false,
    };
    pub const QUIT: Self = Self {
        redraw: false,
        quit: true,
    };
}

/// State mirrored into the native menu's radio/check marks.
#[derive(Clone, Copy, Debug, Default)]
pub struct MenuState {
    pub difficulty_index: usize,
    pub marks_enabled: bool,
}

/// The presentation side of the window. All methods are safe Rust; only the
/// window module needs to deal with handles and FFI.
pub trait WindowApp {
    fn paint(&mut self, canvas: &mut Canvas, width: i32, height: i32);
    fn on_event(&mut self, event: Event) -> EventResult;
    /// Exact client size required by the current board. A change makes the
    /// native window resize and recenter itself.
    fn preferred_client_size(&self) -> (i32, i32);
    fn menu_state(&self) -> MenuState {
        MenuState::default()
    }
    fn wants_hand_cursor(&self, x: i32, y: i32) -> bool {
        let _ = (x, y);
        false
    }
    fn on_destroy(&mut self) {}
}

struct WindowContext {
    app: Box<dyn WindowApp>,
    game_menu: HMENU,
    arrow_cursor: HCURSOR,
    hand_cursor: HCURSOR,
    tracking_mouse: bool,
}

fn to_wide(value: &str) -> Vec<u16> {
    let mut result: Vec<u16> = value.encode_utf16().collect();
    result.push(0);
    result
}

fn last_error(prefix: &str) -> String {
    format!("{} (Win32 error {}).", prefix, unsafe { GetLastError() })
}

unsafe fn context_do<R>(hwnd: HWND, action: impl FnOnce(&mut WindowContext) -> R) -> Option<R> {
    let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowContext;
    if raw.is_null() {
        None
    } else {
        Some(action(&mut *raw))
    }
}

fn append_menu_text(menu: HMENU, flags: UINT, id: u16, text: &str) -> bool {
    let wide = to_wide(text);
    unsafe { AppendMenuW(menu, flags | MF_STRING, id as usize, wide.as_ptr()) != 0 }
}

fn append_menu_separator(menu: HMENU) -> bool {
    unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null()) != 0 }
}

fn append_popup(parent: HMENU, popup: HMENU, text: &str) -> bool {
    let wide = to_wide(text);
    unsafe { AppendMenuW(parent, MF_POPUP | MF_STRING, popup as usize, wide.as_ptr()) != 0 }
}

fn create_main_menu() -> Result<HMENU, String> {
    unsafe {
        let main = CreateMenu();
        let game = CreatePopupMenu();
        let help = CreatePopupMenu();
        if main.is_null() || game.is_null() || help.is_null() {
            if !main.is_null() {
                DestroyMenu(main);
            }
            if !game.is_null() {
                DestroyMenu(game);
            }
            if !help.is_null() {
                DestroyMenu(help);
            }
            return Err(last_error("Could not create the native menu"));
        }

        let mut okay = true;
        okay &= append_menu_text(
            game,
            MF_STRING,
            CMD_NEW_GAME,
            "&\u{041D}\u{043E}\u{0432}\u{0430}\u{044F} \u{0438}\u{0433}\u{0440}\u{0430}\tF2",
        );
        okay &= append_menu_separator(game);
        okay &= append_menu_text(
            game,
            MF_STRING,
            CMD_BEGINNER,
            "&\u{041D}\u{043E}\u{0432}\u{0438}\u{0447}\u{043E}\u{043A}",
        );
        okay &= append_menu_text(
            game,
            MF_STRING,
            CMD_INTERMEDIATE,
            "&\u{041B}\u{044E}\u{0431}\u{0438}\u{0442}\u{0435}\u{043B}\u{044C}",
        );
        okay &= append_menu_text(
            game,
            MF_STRING,
            CMD_EXPERT,
            "&\u{042D}\u{043A}\u{0441}\u{043F}\u{0435}\u{0440}\u{0442}",
        );
        okay &= append_menu_separator(game);
        okay &= append_menu_text(
            game,
            MF_STRING,
            CMD_MARKS,
            "\u{041E}\u{0442}\u{043C}\u{0435}\u{0442}\u{043A}\u{0430} (?)",
        );
        okay &= append_menu_separator(game);
        okay &= append_menu_text(
            game,
            MF_STRING,
            CMD_EXIT,
            "\u{0412}\u{044B}\u{0445}\u{043E}\u{0434}",
        );
        okay &= append_menu_text(
            help,
            MF_STRING,
            CMD_ABOUT,
            "\u{041E} \u{043F}\u{0440}\u{043E}\u{0433}\u{0440}\u{0430}\u{043C}\u{043C}\u{0435}",
        );

        let game_title = "&\u{0418}\u{0433}\u{0440}\u{0430}";
        let help_title = "&\u{0421}\u{043F}\u{0440}\u{0430}\u{0432}\u{043A}\u{0430}";
        okay &= append_popup(main, game, game_title);
        okay &= append_popup(main, help, help_title);

        if !okay {
            DestroyMenu(help);
            DestroyMenu(game);
            DestroyMenu(main);
            return Err(last_error("Could not populate the native menu"));
        }
        Ok(main)
    }
}

unsafe fn sync_menu(hwnd: HWND, context: &WindowContext) {
    let state = context.app.menu_state();
    let checked = match state.difficulty_index {
        1 => CMD_INTERMEDIATE,
        2 => CMD_EXPERT,
        _ => CMD_BEGINNER,
    };
    CheckMenuRadioItem(
        context.game_menu,
        CMD_BEGINNER as UINT,
        CMD_EXPERT as UINT,
        checked as UINT,
        MF_BYCOMMAND,
    );
    let mark_flags = if state.marks_enabled {
        MF_CHECKED
    } else {
        MF_UNCHECKED
    };
    CheckMenuItem(context.game_menu, CMD_MARKS as UINT, mark_flags);
    DrawMenuBar(hwnd);
}

fn window_client_size(hwnd: HWND) -> Option<(i32, i32)> {
    let mut client = WinRect::default();
    let okay = unsafe { GetClientRect(hwnd, &mut client) };
    if okay == 0 || client.right <= client.left || client.bottom <= client.top {
        None
    } else {
        Some((client.right - client.left, client.bottom - client.top))
    }
}

fn resize_client_and_center(hwnd: HWND, client_width: i32, client_height: i32) -> bool {
    if client_width < 240 || client_height < 240 {
        return false;
    }
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as DWORD;
        let mut outer = WinRect {
            left: 0,
            top: 0,
            right: client_width,
            bottom: client_height,
        };
        if AdjustWindowRectEx(&mut outer, style, 1, 0) == 0 {
            return false;
        }
        let outer_width = outer.right - outer.left;
        let outer_height = outer.bottom - outer.top;
        let area = work_area();
        let available_width = (area.right - area.left - outer_width).max(0);
        let available_height = (area.bottom - area.top - outer_height).max(0);
        let x = area.left + available_width / 2;
        let y = area.top + available_height / 2;
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            x,
            y,
            outer_width,
            outer_height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        ) != 0
    }
}

unsafe fn dispatch_event(hwnd: HWND, event: Event) -> LRESULT {
    let mut result = EventResult::NONE;
    let mut preferred_size = None;
    let _ = context_do(hwnd, |context| {
        result = context.app.on_event(event);
        if matches!(event, Event::Command(_)) {
            sync_menu(hwnd, context);
        }
        preferred_size = Some(context.app.preferred_client_size());
    });

    let mut resized = false;
    if let (Some((target_width, target_height)), Some((client_width, client_height))) =
        (preferred_size, window_client_size(hwnd))
    {
        if target_width != client_width || target_height != client_height {
            resized = resize_client_and_center(hwnd, target_width, target_height);
        }
    }

    if result.redraw || resized {
        InvalidateRect(hwnd, std::ptr::null(), 0);
    }
    if result.quit {
        PostQuitMessage(0);
    }
    0
}

unsafe fn paint_window(hwnd: HWND) {
    let mut paint: PaintStruct = std::mem::zeroed();
    let target = BeginPaint(hwnd, &mut paint);
    if target.is_null() {
        return;
    }

    let mut client = WinRect::default();
    if GetClientRect(hwnd, &mut client) != 0
        && client.right > client.left
        && client.bottom > client.top
    {
        let width = client.right - client.left;
        let height = client.bottom - client.top;
        let memory_dc = CreateCompatibleDC(target);
        let bitmap = CreateCompatibleBitmap(target, width, height);
        if !memory_dc.is_null() && !bitmap.is_null() {
            let old_bitmap = SelectObject(memory_dc, bitmap);
            if !old_bitmap.is_null() {
                {
                    let mut canvas = Canvas::new(memory_dc);
                    let _ = context_do(hwnd, |context| {
                        context.app.paint(&mut canvas, width, height);
                    });
                }
                BitBlt(target, 0, 0, width, height, memory_dc, 0, 0, SRCCOPY);
                SelectObject(memory_dc, old_bitmap);
            }
        }
        if !bitmap.is_null() {
            DeleteObject(bitmap);
        }
        if !memory_dc.is_null() {
            DeleteDC(memory_dc);
        }
    }

    EndPaint(hwnd, &paint);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => {
            paint_window(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEMOVE => {
            let _ = context_do(hwnd, |context| {
                if !context.tracking_mouse {
                    let mut track = TrackMouseEventData {
                        cb_size: std::mem::size_of::<TrackMouseEventData>() as DWORD,
                        dw_flags: TME_LEAVE,
                        hwnd_track: hwnd,
                        dw_hover_time: 0,
                    };
                    if TrackMouseEvent(&mut track) != 0 {
                        context.tracking_mouse = true;
                    }
                }
            });
            let (x, y) = mouse_position(l_param);
            dispatch_event(hwnd, Event::MouseMove { x, y })
        }
        WM_MOUSELEAVE => {
            let _ = context_do(hwnd, |context| {
                context.tracking_mouse = false;
            });
            dispatch_event(hwnd, Event::MouseLeave)
        }
        WM_LBUTTONDOWN => {
            SetFocus(hwnd);
            SetCapture(hwnd);
            let (x, y) = mouse_position(l_param);
            dispatch_event(hwnd, Event::LeftDown { x, y })
        }
        WM_RBUTTONDOWN => {
            SetFocus(hwnd);
            let (x, y) = mouse_position(l_param);
            dispatch_event(hwnd, Event::RightDown { x, y })
        }
        WM_MBUTTONDOWN => {
            SetFocus(hwnd);
            let (x, y) = mouse_position(l_param);
            dispatch_event(hwnd, Event::MiddleDown { x, y })
        }
        WM_LBUTTONUP => {
            let (x, y) = mouse_position(l_param);
            let result = dispatch_event(hwnd, Event::LeftUp { x, y });
            ReleaseCapture();
            result
        }
        WM_RBUTTONUP | WM_MBUTTONUP | WM_CONTEXTMENU => 0,
        WM_COMMAND => {
            let command = (w_param & 0xffff) as u16;
            dispatch_event(hwnd, Event::Command(command))
        }
        WM_SIZE => {
            InvalidateRect(hwnd, std::ptr::null(), 0);
            0
        }
        WM_KEYDOWN => dispatch_event(hwnd, Event::KeyDown(w_param as u32)),
        WM_KILLFOCUS => dispatch_event(hwnd, Event::FocusLost),
        WM_TIMER if w_param == TIMER_ID => dispatch_event(hwnd, Event::Timer),
        WM_SETCURSOR => {
            if (l_param as u32) & 0xffff == HTCLIENT {
                let mut point = WinPoint::default();
                if GetCursorPos(&mut point) != 0 && ScreenToClient(hwnd, &mut point) != 0 {
                    let hand = context_do(hwnd, |context| {
                        context.app.wants_hand_cursor(point.x, point.y)
                    })
                    .unwrap_or(false);
                    let cursor = context_do(hwnd, |context| {
                        if hand {
                            context.hand_cursor
                        } else {
                            context.arrow_cursor
                        }
                    })
                    .unwrap_or(std::ptr::null_mut());
                    if !cursor.is_null() {
                        SetCursor(cursor);
                    }
                    return 1;
                }
            }
            DefWindowProcW(hwnd, message, w_param, l_param)
        }
        WM_DESTROY => {
            KillTimer(hwnd, TIMER_ID);
            let _ = context_do(hwnd, |context| context.app.on_destroy());
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, w_param, l_param),
    }
}

fn mouse_position(l_param: LPARAM) -> (i32, i32) {
    let packed = l_param as u32;
    let x = (packed & 0xffff) as u16 as i16 as i32;
    let y = ((packed >> 16) & 0xffff) as u16 as i16 as i32;
    (x, y)
}

fn work_area() -> WinRect {
    let mut area = WinRect::default();
    let okay = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut area as *mut WinRect as *mut c_void,
            0,
        )
    };
    if okay == 0 || area.right <= area.left || area.bottom <= area.top {
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1);
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1);
        area = WinRect {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
    }
    area
}
/// Run the DPI-aware native window. The event loop never enters
/// user code after an invalid HWND and always tears down the context and menu.
pub fn run_window(
    app: Box<dyn WindowApp>,
    title: &str,
    client_width: i32,
    client_height: i32,
) -> Result<(), String> {
    if client_width < 240 || client_height < 240 {
        return Err("The requested window size is too small.".to_string());
    }

    unsafe {
        let _ = SetProcessDPIAware();
    }

    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    if instance.is_null() {
        return Err(last_error("Could not get the application module handle"));
    }

    let class_name = to_wide("MinesweeperRustNativeWindow");
    let window_name = to_wide(title);
    let arrow_cursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW as LPCWSTR) };
    let hand_cursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_HAND as LPCWSTR) };
    let big_icon = OwnedIcon::create(32)?;
    let small_icon = OwnedIcon::create(16)?;
    let main_menu = create_main_menu()?;

    let window_class = WndClassW {
        style: 0,
        wnd_proc: Some(window_proc),
        class_extra: 0,
        window_extra: 0,
        instance,
        icon: big_icon.0,
        cursor: arrow_cursor,
        background: unsafe { GetSysColorBrush(COLOR_BTNFACE) },
        menu_name: std::ptr::null(),
        class_name: class_name.as_ptr(),
    };
    let atom = unsafe { RegisterClassW(&window_class) };
    if atom == 0 {
        let error = unsafe { GetLastError() };
        if error != CLASS_ALREADY_EXISTS {
            unsafe {
                DestroyMenu(main_menu);
            }
            return Err(format!(
                "Could not register the Minesweeper window class (Win32 error {}).",
                error
            ));
        }
    }

    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let mut outer = WinRect {
        left: 0,
        top: 0,
        right: client_width,
        bottom: client_height,
    };
    if unsafe { AdjustWindowRectEx(&mut outer, style, 1, 0) } == 0 {
        unsafe {
            UnregisterClassW(class_name.as_ptr(), instance);
            DestroyMenu(main_menu);
        }
        return Err(last_error("Could not calculate the window dimensions"));
    }
    let outer_width = outer.right - outer.left;
    let outer_height = outer.bottom - outer.top;
    let area = work_area();
    let x = area.left + ((area.right - area.left - outer_width).max(0)) / 2;
    let y = area.top + ((area.bottom - area.top - outer_height).max(0)) / 2;

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            style,
            x.max(0),
            y.max(0),
            outer_width,
            outer_height,
            std::ptr::null_mut(),
            main_menu,
            instance,
            std::ptr::null_mut(),
        )
    };
    if hwnd.is_null() {
        unsafe {
            UnregisterClassW(class_name.as_ptr(), instance);
            DestroyMenu(main_menu);
        }
        return Err(last_error("Could not create the Minesweeper window"));
    }

    unsafe {
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as WPARAM, big_icon.0 as LPARAM);
        SendMessageW(
            hwnd,
            WM_SETICON,
            ICON_SMALL as WPARAM,
            small_icon.0 as LPARAM,
        );
    }

    let game_menu = unsafe { GetSubMenu(main_menu, 0) };
    let mut context = Box::new(WindowContext {
        app,
        game_menu,
        arrow_cursor,
        hand_cursor,
        tracking_mouse: false,
    });
    let context_pointer = (&mut *context) as *mut WindowContext;
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, context_pointer as isize);
        let _ = context_do(hwnd, |context| sync_menu(hwnd, context));
    }

    let timer = unsafe { SetTimer(hwnd, TIMER_ID, 100, None) };
    if timer == 0 {
        let error = last_error("Could not create the game timer");
        unsafe {
            DestroyWindow(hwnd);
            UnregisterClassW(class_name.as_ptr(), instance);
            DestroyMenu(main_menu);
        }
        return Err(error);
    }

    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
    }

    let mut message: Message = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
        if result == -1 {
            unsafe {
                DestroyWindow(hwnd);
                UnregisterClassW(class_name.as_ptr(), instance);
                DestroyMenu(main_menu);
            }
            return Err(last_error("The Windows message loop failed"));
        }
        if result == 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    unsafe {
        KillTimer(hwnd, TIMER_ID);
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        UnregisterClassW(class_name.as_ptr(), instance);
        DestroyMenu(main_menu);
    }
    drop(context);
    Ok(())
}

/// Show the classic Help > About dialog from the native menu.
pub fn show_about() {
    let text = to_wide(
        "\u{0421}\u{0430}\u{043f}\u{0451}\u{0440}\n\n\
         \u{041a}\u{043b}\u{0430}\u{0441}\u{0441}\u{0438}\u{0447}\u{0435}\u{0441}\u{043a}\u{0430}\u{044f} \
         \u{0438}\u{0433}\u{0440}\u{0430} \u{00ab}\u{0421}\u{0430}\u{043f}\u{0451}\u{0440}\u{00bb}.\n\
         \u{0420}\u{0435}\u{0430}\u{043b}\u{0438}\u{0437}\u{0430}\u{0446}\u{0438}\u{044f} \u{043d}\u{0430} Rust \
         \u{0441} \u{0438}\u{043d}\u{0442}\u{0435}\u{0440}\u{0444}\u{0435}\u{0439}\u{0441}\u{043e}\u{043c} Win32/GDI.\n\n\
         \u{00a9} 2026",
    );
    let caption = to_wide(
        "\u{041e} \u{043f}\u{0440}\u{043e}\u{0433}\u{0440}\u{0430}\u{043c}\u{043c}\u{0435}",
    );
    unsafe {
        MessageBoxW(
            GetActiveWindow(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
/// Display a native error dialog. Used only by `main` before/after the loop.
pub fn show_error(message: &str) {
    let text = to_wide(message);
    let caption = to_wide(
        "\u{0421}\u{0430}\u{043f}\u{0451}\u{0440} \u{2014} \u{043e}\u{0448}\u{0438}\u{0431}\u{043a}\u{0430}",
    );
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
