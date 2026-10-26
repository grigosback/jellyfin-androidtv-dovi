//! Dolby Vision profile 7 -> 8.1 sample rewriting for ExoPlayer, the same conversion Kodi does
//! with `videoplayer.convertdovi`: every RPU NAL (type 62) is converted with libdovi's mode 2
//! (8.1, mapping removed) and every enhancement layer NAL (type 63) is dropped, so a profile 8
//! decoder only ever sees the HDR10 base layer plus profile 8.1 metadata.
//!
//! Hybrid releases also carry HDR10+ in every frame. Some TVs (MediaTek based Philips) switch
//! from Dolby Vision to HDR10+ when they see it, so it can be stripped while a Dolby Vision
//! decoder is in use, like Kodi does when HDR10+ is not an allowed HDR format.

use dolby_vision::rpu::dovi_rpu::DoviRpu;
use dolby_vision::rpu::ConversionMode;
use jni::objects::{JByteBuffer, JClass};
use jni::sys::{jboolean, jint, JNI_TRUE};
use jni::JNIEnv;

const NAL_SEI_PREFIX: u8 = 39;
const NAL_UNSPEC62_RPU: u8 = 62;
const NAL_UNSPEC63_EL: u8 = 63;
const START_CODE: [u8; 4] = [0, 0, 0, 1];
const SEI_USER_DATA_REGISTERED_ITU_T_T35: usize = 4;

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

fn clear_emulation_prevention(nal: &[u8]) -> Vec<u8> {
	let mut out = Vec::with_capacity(nal.len());
	for (i, &b) in nal.iter().enumerate() {
		if !(i >= 2 && b == 3 && nal[i - 2] == 0 && nal[i - 1] == 0) {
			out.push(b);
		}
	}
	out
}

fn add_emulation_prevention(rbsp: &[u8]) -> Vec<u8> {
	let mut out = Vec::with_capacity(rbsp.len() + 8);
	let mut zeros = 0;
	for &b in rbsp {
		if zeros >= 2 && b <= 3 {
			out.push(3);
			zeros = 0;
		}
		out.push(b);
		zeros = if b == 0 { zeros + 1 } else { 0 };
	}
	out
}

/// ST 2094-40 (HDR10+): ITU-T T.35 United States, Samsung Electronics America, application 4.
fn is_hdr10plus(payload: &[u8]) -> bool {
	payload.len() >= 7 && payload[..5] == [0xb5, 0x00, 0x3c, 0x00, 0x01] && payload[5] == 4 && payload[6] <= 1
}

fn read_sei_value(rbsp: &[u8], pos: &mut usize) -> Option<usize> {
	let mut value = 0;
	loop {
		let b = *rbsp.get(*pos)?;
		*pos += 1;
		value += b as usize;
		if b != 0xff {
			return Some(value);
		}
	}
}

/// Removes HDR10+ messages from a prefix SEI NAL. Returns None when there is nothing to remove
/// (or the NAL can't be parsed) and an empty Vec when HDR10+ was the only message.
fn strip_hdr10plus(nal: &[u8]) -> Option<Vec<u8>> {
	let rbsp = clear_emulation_prevention(nal);
	let mut kept = rbsp.get(..2)?.to_vec();
	let mut found = false;
	let mut pos = 2;

	// The last byte is rbsp_trailing_bits
	while rbsp.len() - pos > 1 {
		let start = pos;
		let payload_type = read_sei_value(&rbsp, &mut pos)?;
		let payload_size = read_sei_value(&rbsp, &mut pos)?;
		let payload = rbsp.get(pos..pos + payload_size)?;
		pos += payload_size;

		if payload_type == SEI_USER_DATA_REGISTERED_ITU_T_T35 && is_hdr10plus(payload) {
			found = true;
		} else {
			kept.extend_from_slice(&rbsp[start..pos]);
		}
	}

	if !found {
		return None;
	}
	if kept.len() == 2 {
		return Some(Vec::new());
	}
	kept.extend_from_slice(&rbsp[pos..]);
	Some(add_emulation_prevention(&kept))
}

/// Rewrites one Annex-B access unit. Returns None when the sample has no RPU or EL NALs, so
/// non-DV and already-converted samples are passed through without copying.
pub fn rewrite_annexb(data: &[u8], remove_hdr10plus: bool) -> Option<Vec<u8>> {
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
			NAL_SEI_PREFIX if remove_hdr10plus => match strip_hdr10plus(nal) {
				Some(stripped) if stripped.is_empty() => continue,
				Some(stripped) => {
					out.extend_from_slice(&START_CODE);
					out.extend_from_slice(&stripped);
				}
				None => {
					out.extend_from_slice(&START_CODE);
					out.extend_from_slice(nal);
				}
			},
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
/// HDR10+ SEI messages are removed too when `remove_hdr10plus` is set.
#[no_mangle]
pub extern "system" fn Java_org_jellyfin_playback_media3_exoplayer_dovi_DoviNative_rewriteSample(
	env: JNIEnv,
	_class: JClass,
	buffer: JByteBuffer,
	length: jint,
	capacity: jint,
	remove_hdr10plus: jboolean,
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
	match rewrite_annexb(&data[..length as usize], remove_hdr10plus == JNI_TRUE) {
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
		assert_eq!(rewrite_annexb(&sample, true), None);
	}

	#[test]
	fn drops_enhancement_layer() {
		let sample = [0, 0, 0, 1, 0x26, 0x01, 0xaa, 0, 0, 0, 1, 0x7e, 0x01, 0x40, 0x01, 0xdd];
		let out = rewrite_annexb(&sample, false).unwrap();
		assert_eq!(out, vec![0, 0, 0, 1, 0x26, 0x01, 0xaa]);
	}

	const HDR10PLUS_MSG: [u8; 9] = [0x04, 0x07, 0xb5, 0x00, 0x3c, 0x00, 0x01, 0x04, 0x01];
	const MDCV_MSG: [u8; 4] = [0x89, 0x02, 0x00, 0x00];

	#[test]
	fn removes_sei_with_only_hdr10plus() {
		let mut sei = vec![0x4e, 0x01];
		sei.extend_from_slice(&HDR10PLUS_MSG);
		sei.push(0x80);
		assert_eq!(strip_hdr10plus(&sei), Some(Vec::new()));
	}

	#[test]
	fn keeps_other_sei_messages() {
		let mut sei = vec![0x4e, 0x01];
		sei.extend_from_slice(&MDCV_MSG);
		sei.extend_from_slice(&HDR10PLUS_MSG);
		sei.push(0x80);
		assert_eq!(strip_hdr10plus(&sei), Some(vec![0x4e, 0x01, 0x89, 0x02, 0x00, 0x00, 0x80]));

		let mut other = vec![0x4e, 0x01];
		other.extend_from_slice(&MDCV_MSG);
		other.push(0x80);
		assert_eq!(strip_hdr10plus(&other), None);
	}

	#[test]
	fn emulation_prevention_round_trip() {
		let nal = [0x4e, 0x01, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x03, 0x00, 0x80];
		let rbsp = clear_emulation_prevention(&nal);
		assert_eq!(rbsp, vec![0x4e, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80]);
		assert_eq!(add_emulation_prevention(&rbsp), nal.to_vec());
	}

	#[test]
	fn hdr10plus_only_removed_when_asked() {
		let mut sample = vec![0, 0, 0, 1, 0x4e, 0x01];
		sample.extend_from_slice(&HDR10PLUS_MSG);
		sample.extend_from_slice(&[0x80, 0, 0, 0, 1, 0x7e, 0x01, 0x40, 0, 0, 0, 1, 0x26, 0x01, 0xaa]);
		assert_eq!(rewrite_annexb(&sample, true).unwrap(), vec![0, 0, 0, 1, 0x26, 0x01, 0xaa]);
		assert_eq!(rewrite_annexb(&sample, false).unwrap().len(), 4 + 2 + HDR10PLUS_MSG.len() + 1 + 7);
	}
}
