package org.jellyfin.playback.media3.exoplayer.dovi

import android.content.Context
import android.os.Handler
import androidx.annotation.OptIn
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.Renderer
import androidx.media3.exoplayer.mediacodec.MediaCodecSelector
import androidx.media3.exoplayer.video.MediaCodecVideoRenderer
import androidx.media3.exoplayer.video.VideoRendererEventListener

/**
 * A [DefaultRenderersFactory] that uses [DoviVideoRenderer] instead of the default
 * [MediaCodecVideoRenderer].
 */
@OptIn(UnstableApi::class)
class DoviRenderersFactory(context: Context) : DefaultRenderersFactory(context) {
	override fun buildVideoRenderers(
		context: Context,
		extensionRendererMode: Int,
		mediaCodecSelector: MediaCodecSelector,
		enableDecoderFallback: Boolean,
		eventHandler: Handler,
		eventListener: VideoRendererEventListener,
		allowedVideoJoiningTimeMs: Long,
		out: ArrayList<Renderer>,
	) {
		super.buildVideoRenderers(
			context,
			extensionRendererMode,
			mediaCodecSelector,
			enableDecoderFallback,
			eventHandler,
			eventListener,
			allowedVideoJoiningTimeMs,
			out,
		)

		val index = out.indexOfFirst { it.javaClass == MediaCodecVideoRenderer::class.java }
		if (index == -1) return

		out[index] = DoviVideoRenderer(
			MediaCodecVideoRenderer.Builder(context)
				.setCodecAdapterFactory(codecAdapterFactory)
				.setMediaCodecSelector(mediaCodecSelector)
				.setAllowedJoiningTimeMs(allowedVideoJoiningTimeMs)
				.setEnableDecoderFallback(enableDecoderFallback)
				.setEventHandler(eventHandler)
				.setEventListener(eventListener)
				.setMaxDroppedFramesToNotify(MAX_DROPPED_VIDEO_FRAME_COUNT_TO_NOTIFY)
		)
	}
}
