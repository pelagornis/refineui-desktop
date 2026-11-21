# Material Engine benchmark — 2026-11-20

- OS: Linux 6.12.113+ x86_64
- CPU: Intel(R) Xeon(R) Processor (8 threads)
- Vulkan: unknown
- Display: :141
- rustc: rustc 1.100.0 (0ef1bc386 2026-11-09)
- Command: `target/release/examples/blur_bench <panels>` (cache on) and `GPUI_BLUR_CACHE=0 target/release/examples/blur_bench <panels>` (cache off)

| Panels | Cache | blur/moving | blur/static | fallback |
| --- | --- | --- | --- | --- |
| 1 | on | 15.98 ms (62.6 fps) | 15.96 ms (62.7 fps) | 15.96 ms (62.7 fps) |
| 1 | off | 16.06 ms (62.3 fps) | 15.96 ms (62.7 fps) | 15.96 ms (62.7 fps) |
| 8 | on | 16.88 ms (59.3 fps) | 15.96 ms (62.7 fps) | 16.04 ms (62.3 fps) |
| 8 | off | 16.23 ms (61.6 fps) | 15.96 ms (62.7 fps) | 15.96 ms (62.7 fps) |
| 16 | on | 32.30 ms (31.0 fps) | 16.04 ms (62.3 fps) | 16.57 ms (60.3 fps) |
| 16 | off | 32.13 ms (31.1 fps) | 31.92 ms (31.3 fps) | 15.96 ms (62.7 fps) |

<details><summary>Raw output</summary>

```

## 1 panels, cache on
blur cache: on
blur/moving  panels=1  frames=188   avg=  15.98 ms  fps=   62.6
blur/static  panels=1  frames=189   avg=  15.96 ms  fps=   62.7
fallback     panels=1  frames=189   avg=  15.96 ms  fps=   62.7

## 1 panels, cache off
blur cache: off
blur/moving  panels=1  frames=187   avg=  16.06 ms  fps=   62.3
blur/static  panels=1  frames=189   avg=  15.96 ms  fps=   62.7
fallback     panels=1  frames=189   avg=  15.96 ms  fps=   62.7

## 8 panels, cache on
blur cache: on
blur/moving  panels=8  frames=178   avg=  16.88 ms  fps=   59.3
blur/static  panels=8  frames=188   avg=  15.96 ms  fps=   62.7
fallback     panels=8  frames=187   avg=  16.04 ms  fps=   62.3

## 8 panels, cache off
blur cache: off
blur/moving  panels=8  frames=185   avg=  16.23 ms  fps=   61.6
blur/static  panels=8  frames=188   avg=  15.96 ms  fps=   62.7
fallback     panels=8  frames=188   avg=  15.96 ms  fps=   62.7

## 16 panels, cache on
blur cache: on
blur/moving  panels=16 frames=93    avg=  32.30 ms  fps=   31.0
blur/static  panels=16 frames=188   avg=  16.04 ms  fps=   62.3
fallback     panels=16 frames=181   avg=  16.57 ms  fps=   60.3

## 16 panels, cache off
blur cache: off
blur/moving  panels=16 frames=94    avg=  32.13 ms  fps=   31.1
blur/static  panels=16 frames=95    avg=  31.92 ms  fps=   31.3
fallback     panels=16 frames=188   avg=  15.96 ms  fps=   62.7

```

</details>
