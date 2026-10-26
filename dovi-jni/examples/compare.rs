//! Checks `rewrite_annexb` against dovi_tool: `compare <input.hevc> <dovi_tool -m 2 convert --discard output>`.
//! Prints NAL counts and whether the NAL payload sequences are identical.

fn nals(data: &[u8]) -> Vec<&[u8]> {
	let mut starts = Vec::new();
	let mut i = 0;
	while i + 3 <= data.len() {
		if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
			starts.push(i + 3);
			i += 3;
		} else {
			i += 1;
		}
	}
	let mut out = Vec::new();
	for (n, &s) in starts.iter().enumerate() {
		let mut e = starts.get(n + 1).map(|&x| x - 3).unwrap_or(data.len());
		while e > s && data[e - 1] == 0 {
			e -= 1;
		}
		out.push(&data[s..e]);
	}
	out
}

fn main() {
	let args: Vec<String> = std::env::args().collect();
	let input = std::fs::read(&args[1]).unwrap();
	let reference = std::fs::read(&args[2]).unwrap();
	let t = std::time::Instant::now();
	let ours = dovi_jni::rewrite_annexb(&input, std::env::var("STRIP_HDR10PLUS").is_ok()).expect("nothing rewritten");
	let elapsed = t.elapsed();
	if let Ok(path) = std::env::var("OUT") {
		std::fs::write(path, &ours).unwrap();
	}
	let (a, b) = (nals(&ours), nals(&reference));
	let count = |v: &Vec<&[u8]>, t: u8| v.iter().filter(|n| (n[0] >> 1) & 0x3f == t).count();
	println!("input {} bytes -> ours {} bytes in {:?}", input.len(), ours.len(), elapsed);
	println!("ours: {} NALs, {} RPU, {} EL", a.len(), count(&a, 62), count(&a, 63));
	println!("ref : {} NALs, {} RPU, {} EL", b.len(), count(&b, 62), count(&b, 63));
	let diffs: Vec<usize> = (0..a.len().min(b.len())).filter(|&i| a[i] != b[i]).collect();
	println!("differing NALs: {} (first: {:?})", diffs.len(), diffs.first());
	if let Some(&i) = diffs.first() {
		println!("  type ours {} ref {}", (a[i][0] >> 1) & 0x3f, (b[i][0] >> 1) & 0x3f);
	}
	println!("identical: {}", a == b);
}
