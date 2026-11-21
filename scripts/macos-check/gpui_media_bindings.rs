// Stub bindgen output for scripts/macos-check (the real file needs the macOS SDK).
// Stub for cross-checking on Linux (bindgen output needs the macOS SDK).
pub type CMItemIndex = isize;
pub type CMVideoCodecType = u32;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CMTime {
    pub value: i64,
    pub timescale: i32,
    pub flags: u32,
    pub epoch: i64,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CMSampleTimingInfo {
    pub duration: CMTime,
    pub presentationTimeStamp: CMTime,
    pub decodeTimeStamp: CMTime,
}
#[repr(C)]
pub struct __CFString {
    _p: [u8; 0],
}
unsafe extern "C" {
    pub static kCMTimeInvalid: CMTime;
    pub static kCMSampleAttachmentKey_NotSync: *const __CFString;
    pub fn CMTimeMake(value: i64, timescale: i32) -> CMTime;
}
pub const kCMVideoCodecType_H264: u32 = 1635148593;
pub type CVReturn = i32;
pub const kCVReturnSuccess: CVReturn = 0;
pub const kCVPixelFormatType_32BGRA: u32 = 1111970369;
pub const kCVPixelFormatType_420YpCbCr8BiPlanarFullRange: u32 = 875704422;
pub const kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange: u32 = 875704438;
pub const kCVPixelFormatType_420YpCbCr8Planar: u32 = 2033463856;
