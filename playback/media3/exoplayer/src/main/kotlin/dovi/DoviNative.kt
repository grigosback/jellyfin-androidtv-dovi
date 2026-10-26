package org.jellyfin.playback.media3.exoplayer.dovi

import timber.log.Timber
import java.nio.ByteBuffer

/**
 * Bindings for libdovi_jni (source in /dovi-jni), which converts Dolby Vision profile 7 samples
 * to profile 8.1 the same way Kodi does: RPUs are converted with libdovi mode 2 and enhancement
 * layer NALs are dropped.
 */
object DoviNative {
	val isAvailable: Boolean = try {
		System.loadLibrary("dovi_jni")
		true
	} catch (err: UnsatisfiedLinkError) {
		Timber.w(err, "Dolby Vision profile 7 conversion unavailable")
		false
	}

	/**
	 * Rewrites the first [length] bytes of the direct [buffer] (an Annex-B access unit) in place.
	 *
	 * @return the new length, or -1 when the buffer was left untouched.
	 */
	@JvmStatic
	external fun rewriteSample(buffer: ByteBuffer, length: Int, capacity: Int): Int
}
