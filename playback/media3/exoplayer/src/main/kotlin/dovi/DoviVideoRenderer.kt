package org.jellyfin.playback.media3.exoplayer.dovi

import android.media.MediaCodecInfo.CodecProfileLevel
import androidx.annotation.OptIn
import androidx.media3.common.Format
import androidx.media3.common.MimeTypes
import androidx.media3.common.util.UnstableApi
import androidx.media3.decoder.DecoderInputBuffer
import androidx.media3.exoplayer.DecoderReuseEvaluation
import androidx.media3.exoplayer.FormatHolder
import androidx.media3.exoplayer.mediacodec.MediaCodecInfo
import androidx.media3.exoplayer.mediacodec.MediaCodecSelector
import androidx.media3.exoplayer.mediacodec.MediaCodecUtil
import androidx.media3.exoplayer.video.MediaCodecVideoRenderer
import timber.log.Timber

/**
 * A [MediaCodecVideoRenderer] that plays Dolby Vision profile 7 (dual layer, as found on UHD
 * Blu-ray remuxes) on decoders that only support profile 8, like Kodi's "convert Dolby Vision"
 * option: the track is announced to the decoder as profile 8.1 and every sample is rewritten by
 * [DoviNative] before it is queued.
 *
 * Devices with a real profile 7 decoder keep the original stream.
 */
@OptIn(UnstableApi::class)
class DoviVideoRenderer(builder: Builder) : MediaCodecVideoRenderer(builder) {
	private var convertSamples = false

	override fun supportsFormat(mediaCodecSelector: MediaCodecSelector, format: Format): Int =
		super.supportsFormat(mediaCodecSelector, format.toProfile81OrSelf())

	override fun getDecoderInfos(
		mediaCodecSelector: MediaCodecSelector,
		format: Format,
		requiresSecureDecoder: Boolean,
	): List<MediaCodecInfo> = super.getDecoderInfos(mediaCodecSelector, format.toProfile81OrSelf(), requiresSecureDecoder)

	override fun onInputFormatChanged(formatHolder: FormatHolder): DecoderReuseEvaluation? {
		val format = formatHolder.format
		val converted = format?.toProfile81OrSelf()
		convertSamples = converted !== format
		if (convertSamples) {
			Timber.i("Converting Dolby Vision profile 7 to 8.1 (${format?.codecs} -> ${converted?.codecs})")
			formatHolder.format = converted
		}
		return super.onInputFormatChanged(formatHolder)
	}

	override fun onQueueInputBuffer(buffer: DecoderInputBuffer) {
		val data = buffer.data
		if (convertSamples && data != null && data.isDirect && !data.isReadOnly && data.position() == 0) {
			val length = DoviNative.rewriteSample(data, data.limit(), data.capacity())
			if (length >= 0) data.limit(length)
		}
		super.onQueueInputBuffer(buffer)
	}

	private companion object {
		private val supportsProfile7Natively: Boolean by lazy {
			runCatching {
				MediaCodecUtil.getDecoderInfos(MimeTypes.VIDEO_DOLBY_VISION, false, false).any { info ->
					info.profileLevels.any { it.profile == CodecProfileLevel.DolbyVisionProfileDvheDtb }
				}
			}.getOrDefault(false)
		}

		private fun Format.isDoviProfile7() = sampleMimeType == MimeTypes.VIDEO_DOLBY_VISION &&
			codecs?.startsWith("dvhe.07") == true

		/** Returns this format relabelled as profile 8.1 when samples need converting, otherwise itself. */
		fun Format.toProfile81OrSelf(): Format {
			if (!isDoviProfile7() || !DoviNative.isAvailable || supportsProfile7Natively) return this

			return buildUpon()
				.setCodecs(codecs?.replaceFirst("dvhe.07", "dvhe.08"))
				.build()
		}
	}
}
