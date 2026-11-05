import { useEffect, useImperativeHandle, useRef, forwardRef, useState } from "react";
import Hls from "hls.js";
import { LOCAL, nativeHls, playlist } from "../api";

export type VideoHandle = {
  el: () => HTMLVideoElement | null;
  play: () => void;
  pause: () => void;
  toggle: () => void;
  seek: (t: number) => void;
  rate: (r: number) => void;
  mute: (m: boolean) => void;
  volume: (v: number) => void;
};

type Props = {
  src: string;
  poster?: string;
  autoPlay?: boolean;
  muted?: boolean;
  loop?: boolean;
  fit?: "contain" | "cover";
  /** 换源后重新套用，避免切集时音量/倍速被浏览器重置 */
  volume?: number;
  rate?: number;
  onTime?: (t: number) => void;
  onDuration?: (d: number) => void;
  onSize?: (w: number, h: number) => void;
  onPlaying?: () => void;
  onPause?: () => void;
  onWaiting?: () => void;
  onEnded?: () => void;
  onError?: (msg: string) => void;
  onClick?: () => void;
  className?: string;
};

/** 只负责「播」：hls.js + video 元素，控制条由外层渲染。 */
const VideoSurface = forwardRef<VideoHandle, Props>(function VideoSurface(
  { src, poster, autoPlay = true, muted = false, loop = false, fit = "contain", volume = 1, rate = 1,
    onTime, onDuration, onSize, onPlaying, onPause, onWaiting, onEnded, onError, onClick, className = "" },
  ref,
) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const hlsRef = useRef<Hls | null>(null);
  const [ready, setReady] = useState(false);

  useImperativeHandle(ref, () => ({
    el: () => videoRef.current,
    play: () => { videoRef.current?.play().catch(() => {}); },
    pause: () => videoRef.current?.pause(),
    toggle: () => {
      const v = videoRef.current;
      if (!v) return;
      if (v.paused) v.play().catch(() => {}); else v.pause();
    },
    seek: (t: number) => { const v = videoRef.current; if (v && Number.isFinite(t)) v.currentTime = t; },
    rate: (r: number) => { const v = videoRef.current; if (v) v.playbackRate = r; },
    mute: (m: boolean) => { const v = videoRef.current; if (v) v.muted = m; },
    volume: (vol: number) => { const v = videoRef.current; if (v) v.volume = vol; },
  }), []);

  useEffect(() => {
    const video = videoRef.current;
    if (!video || !src) return;
    setReady(false);
    if (hlsRef.current) { hlsRef.current.destroy(); hlsRef.current = null; }

    const proxied = src.startsWith(`${LOCAL}/`);
    const url = proxied ? src : playlist(src);
    const useHls = !url.includes("/seg?u=");

    /* 换源时先把音量/倍速套回去：hls 重新 attach 后浏览器会把它们重置成默认值 */
    const applySettings = () => {
      const v = videoRef.current;
      if (!v) return;
      v.volume = volume;
      v.muted = muted;
      v.playbackRate = rate;
    };
    applySettings();

    let stopFallback = () => {};
    const startHls = () => {
      const hls = new Hls({
        enableWorker: true,
        maxBufferLength: 30,
        backBufferLength: 30,
        manifestLoadingTimeOut: 20000,
        manifestLoadingMaxRetry: 3,
        fragLoadingMaxRetry: 4,
      });
      hlsRef.current = hls;
      hls.loadSource(url);
      hls.attachMedia(video);
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        if (autoPlay) video.play().catch(() => {});
      });
      hls.on(Hls.Events.ERROR, (_e, data) => {
        if (!data.fatal) return;
        onError?.(`${data.type}/${data.details}`);
        if (data.type === Hls.ErrorTypes.NETWORK_ERROR) hls.startLoad();
        else if (data.type === Hls.ErrorTypes.MEDIA_ERROR) hls.recoverMediaError();
        else { hls.destroy(); hlsRef.current = null; }
      });
    };
    if (useHls && !nativeHls && Hls.isSupported()) {
      startHls();
    } else {
      video.src = url;
      if (autoPlay) video.play().catch(() => {});

      // iOS 原生 HLS 若打不开（报错，或开始播放后 8 秒还没出画面），自动退回 hls.js，保证能播
      if (nativeHls && Hls.isSupported() && useHls) {
        let settled = false;
        const fallback = () => {
          if (settled) return;
          settled = true;
          video.removeAttribute("src");
          video.load();
          startHls();
        };
        let timer = 0;
        const ok = () => { settled = true; window.clearTimeout(timer); };
        // 只在真正要播放时计时，避免预加载的视频被误判
        const armed = () => {
          window.clearTimeout(timer);
          if (!settled && video.readyState < 2) timer = window.setTimeout(fallback, 8000);
        };
        video.addEventListener("error", fallback, { once: true });
        video.addEventListener("loadeddata", ok, { once: true });
        video.addEventListener("play", armed);
        if (!video.paused) armed();
        stopFallback = () => {
          window.clearTimeout(timer);
          video.removeEventListener("play", armed);
          video.removeEventListener("error", fallback);
          video.removeEventListener("loadeddata", ok);
        };
      }
    }

    return () => {
      stopFallback();
      if (hlsRef.current) { hlsRef.current.destroy(); hlsRef.current = null; }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [src]);

  useEffect(() => { if (videoRef.current) videoRef.current.muted = muted; }, [muted]);
  useEffect(() => { if (videoRef.current) videoRef.current.volume = volume; }, [volume]);
  useEffect(() => { if (videoRef.current) videoRef.current.playbackRate = rate; }, [rate]);

  const reapply = (v: HTMLVideoElement) => {
    v.volume = volume;
    v.muted = muted;
    v.playbackRate = rate;
  };

  return (
    <video
      ref={videoRef}
      poster={poster}
      playsInline
      loop={loop}
      onClick={onClick}
      className={`w-full h-full bg-black ${fit === "cover" ? "object-cover" : "object-contain"} ${className}`}
      style={{ opacity: ready ? 1 : 0, transition: "opacity .35s" }}
      onLoadedData={(e) => { setReady(true); reapply(e.target as HTMLVideoElement); }}
      onTimeUpdate={(e) => onTime?.((e.target as HTMLVideoElement).currentTime)}
      onLoadedMetadata={(e) => {
        const v = e.target as HTMLVideoElement;
        reapply(v);
        onDuration?.(v.duration || 0);
        onSize?.(v.videoWidth || 0, v.videoHeight || 0);
      }}
      onPlay={() => onPlaying?.()}
      onPause={() => onPause?.()}
      onWaiting={() => onWaiting?.()}
      onPlaying={() => onPlaying?.()}
      onEnded={() => onEnded?.()}
    />
  );
});

export default VideoSurface;