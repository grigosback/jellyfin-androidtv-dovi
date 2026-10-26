//! Dolby Vision profile 7 -> 8.1 sample rewriting for ExoPlayer, the same conversion Kodi does
//! with `videoplayer.convertdovi`: every RPU NAL (type 62) is converted with libdovi's mode 2
//! (8.1, mapping removed) and every enhancement layer NAL (type 63) is dropped, so a profile 8
//! decoder only ever sees the HDR10 base layer plus profile 8.1 metadata.

use dolby_vision::rpu::dovi_rpu::DoviRpu;
use dolby_vision::rpu::ConversionMode;
use jni::objects::{JByteBuffer, JClass};
use jni::sys::jint;
use jni::JNIEnv;

const NAL_UNSPEC62_RPU: u8 = 62;
const NAL_UNSPEC63_EL: u8 = 63;
const START_CODE: [u8; 4] = [0, 0, 0, 1];

/// Returns the (start, end) payload ranges of every NAL unit in an Annex-B buffer.
/// Zero bytes that belong to the next 4-byte start code are not part of the previous NAL.
fn nal_ranges(data: &[u8]) -> Vec<(usize, usize)> {
	let mut starts = Vec::new();
	let mut i = 0;
	while i + 3 <= data.len() {
		if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
			starts.push((i, i + 3));
			i += 3;
		} else {
			i += 1;
		}
	}

	let mut ranges = Vec::with_capacity(starts.len());
	for (n, &(_, payload_start)) in starts.iter().enumerate() {
		let mut end = match starts.get(n + 1) {
			Some(&(next_code, _)) => next_code,
			None => data.len(),
		};
		while end > payload_start && data[end - 1] == 0 {
			end -= 1;
		}
		if end > payload_start {
			ranges.push((payload_start, end));
		}
	}
	ranges
}

fn nal_type(nal: &[u8]) -> u8 {
	(nal[0] >> 1) & 0x3f
}

/// Converts one RPU NAL to profile 8.1. Returns None when it is not a profile 7 RPU or libdovi
/// fails, in which case the caller keeps the original NAL (same fallback as Kodi).
fn convert_rpu(nal: &[u8]) -> Option<Vec<u8>> {
	let mut rpu = DoviRpu::parse_unspec62_nalu(nal).ok()?;
	if rpu.dovi_profile != 7 {
		return None;
	}
	rpu.convert_with_mode(ConversionMode::To81).ok()?;
	rpu.write_hevc_unspec62_nalu().ok()
}

/// Rewrites one Annex-B access unit. Returns None when the sample has no RPU or EL NALs, so
/// non-DV and already-converted samples are passed through without copying.
pub fn rewrite_annexb(data: &[u8]) -> Option<Vec<u8>> {
	let ranges = nal_ranges(data);
	if !ranges
		.iter()
		.any(|&(s, _)| matches!(nal_type(&data[s..]), NAL_UNSPEC62_RPU | NAL_UNSPEC63_EL))
	{
		return None;
	}

	let mut out = Vec::with_capacity(data.len());
	for (s, e) in ranges {
		let nal = &data[s..e];
		match nal_type(nal) {
			NAL_UNSPEC63_EL => continue,
			NAL_UNSPEC62_RPU => {
				out.extend_from_slice(&START_CODE);
				match convert_rpu(nal) {
					Some(converted) => out.extend_from_slice(&converted),
					None => out.extend_from_slice(nal),
				}
			}
			_ => {
				out.extend_from_slice(&START_CODE);
				out.extend_from_slice(nal);
			}
		}
	}
	Some(out)
}

/// JNI entry point for `DoviNative.rewriteSample`. Rewrites `length` bytes of the direct
/// `buffer` in place and returns the new length, or -1 when the buffer is left untouched
/// (nothing to rewrite, not a direct buffer, or the result would not fit in `capacity`).
#[no_mangle]
pub extern "system" fn Java_org_jellyfin_playback_media3_exoplayer_dovi_DoviNative_rewriteSample(
	env: JNIEnv,
	_class: JClass,
	buffer: JByteBuffer,
	length: jint,
	capacity: jint,
) -> jint {
	let Ok(ptr) = env.get_direct_buffer_address(&buffer) else {
		return -1;
	};
	if ptr.is_null() || length <= 0 || capacity < length {
		return -1;
	}

	// SAFETY: the buffer is a direct ByteBuffer of at least `capacity` bytes that is not
	// accessed by anything else while the renderer queues it.
	let data = unsafe { std::slice::from_raw_parts_mut(ptr, capacity as usize) };
	match rewrite_annexb(&data[..length as usize]) {
		Some(out) if out.len() <= capacity as usize => {
			data[..out.len()].copy_from_slice(&out);
			out.len() as jint
		}
		_ => -1,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn passes_through_plain_hevc() {
		let sample = [0, 0, 0, 1, 0x26, 0x01, 0xaa, 0xbb, 0, 0, 0, 1, 0x02, 0x01, 0xcc];
		assert_eq!(rewrite_annexb(&sample), None);
	}

	#[test]
	fn drops_enhancement_layer() {
		let sample = [0, 0, 0, 1, 0x26, 0x01, 0xaa, 0, 0, 0, 1, 0x7e, 0x01, 0x40, 0x01, 0xdd];
		let out = rewrite_annexb(&sample).unwrap();
		assert_eq!(out, vec![0, 0, 0, 1, 0x26, 0x01, 0xaa]);
	}
}
