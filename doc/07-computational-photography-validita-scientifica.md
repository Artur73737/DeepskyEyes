# 07 — Fotografia computazionale e validità scientifica RAW (ricerca online)

Copre: README §51, §52, §94, §95.

## 1. Cosa fa Google di default

- **HDR+**: burst di RAW sottoesposti (2–15 frame) allineati e fusi → computational RAW → finitura. Riduce rumore shot+read, aumenta DR.
- **HDR+ con bracketing** (Pixel 5+): esposizioni diverse fuse per ombre/luci.
- **Night Sight**: motion metering (optical flow) + exposure fino a ~333 ms/frame (meno su selfie/senza OIS), fusione multi-frame. Su Pixel stock, anche con Pro Controls HDR+ resta attivo.
- **Astrophotography mode stock**: ~16×16 s fusi in ~4 min. Non è controllo manuale RAW: è pipeline fusa.
- Paper IPOL 2021 analizza HDR+ burst denoising: modello rumore Poisson-Gaussiano, dataset `merged.dng`.

Implicazione (§51): verificare temporal/spatial denoise, sharpening, HDR, fusione multi-frame,
lens correction, binning, remosaic, tonemap, black-level, hot-pixel su path `RAW_SENSOR` Camera2.
RAW capability ≠ astro-cam dedicata.

## 2. Validità scientifica (§52/§95)

Da misurare: Bayer pattern, bit depth, black/white/saturation, guadagni analogico/digitali,
read noise, dark current, FPN, hot/defective pixel, dipendenza termica, lens shading,
optical black, crop/binning/pixel mode. DNG opzionali di Camera2 aiutano ma serve validazione empirica.

Metodo (§58): 100×15 s, statistiche mean/median/variance/black/saturazione/istogramma/hot-pixel,
cerca variazioni frame-to-frame e non linearità (ISO invariance, linearità sensore/gain, DR, amp glow).

## 3. Checklist (§94–95)

Denoise RAW? hot-pixel? shading? sharpening? HDR? fusione? processing nascosto? black-level?
calibrazione automatica? vendor behavior? + read/dark/thermal/FPN/hot/amp-glow/linearità/saturazione/
DR/ISO-invariance/dipendenza temperatura/long-exposure.

## Fonti

- https://research.google/blog/hdr-low-light-and-high-dynamic-range-photography-in-the-google-camera-app
- https://research.google/blog/hdr-with-bracketing-on-pixel-phones
- https://research.google/blog/night-sight-seeing-in-the-dark-on-pixel-phones
- https://arxiv.org/pdf/2110.09354 (HDR+ burst denoising, IPOL 2021)
- https://blog.google/products-and-platforms/devices/pixel/google-pixel-8-pro-camera
- https://www.androidpolice.com/how-to-manually-turn-on-astrophotography-mode-on-pixel
