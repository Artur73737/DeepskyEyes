# 01 — Pixel 8 Pro: hardware fotografico reale (ricerca online)

Copre: README §1, §50, §90, §122 Phase 0.

## 1. Scheda ufficiale (Google Store)

- **SoC:** Google Tensor G3 + Titan M2. RAM 12 GB LPDDR5X, storage UFS 3.1 128/256/512 GB/1 TB.
- **Display:** 6.7" LTPO OLED 1344×2992, 1–120 Hz, fino a 1600 nits HDR / 2400 picco.
- **Batteria:** tipica 5050 mAh (min 4950). Ricarica 30 W USB-PD 3.0 PPS (~50% in 30'), wireless Qi, Battery Share.
- **USB:** USB-C **3.2** (rilevante per throughput RAW, §60).
- **Fotocamere posteriori:**
  - Wide 50 MP Octa PD, pixel 1.2 µm, f/1.68, 82° FoV, sensore 1/1.31", OIS+EIS;
  - Ultrawide 48 MP Quad PD con AF, 0.8 µm, f/1.95, 125.5° FoV, Lens correction;
  - Tele 48 MP Quad PD, 0.7 µm, f/2.8, 21.8° FoV, zoom ottico 5x, Super Res fino a 30x, OIS+EIS;
  - Multi-zone LDAF (laser), sensore spettrale+flicker.
- **Frontale:** 10.5 MP Dual PD, 1.22 µm, f/2.2, AF, 95°.
- **Video:** 4K 24/30/60 su tutte le camere, 1080p idem, Dual exposure sulla wide, 10-bit HDR, Night Sight Video / Video Boost, Slo-mo 240 fps.

## 2. Sensori (incrocio recensioni tecniche)

- Main 50 MP Type 1/1.31" (9.8×7.4 mm), Octa-PD, binning 4:1 → 12.5 MP di default. Identificato come **Samsung ISOCELL GN2** (upgrade da GN1 di Pixel 7): ~35% più luce, Staggered HDR, Dual Exposure video.
- Ultrawide: upgrade maggiore della serie — da 12 MP IMX386 a sensore ~2x più grande (leak iniziale: Sony IMX787 64 MP; scheda finale: **48 MP Quad PD**). Macro Focus fino a ~2 cm.
- Tele: 48 MP 1/2.55" 0.7 µm, focale equiv ~112–113 mm, apertura migliorata f/3.5 → **f/2.8**, stabilizzata.
- Nota discrepanze leak vs finale: i leak pre-lancio parlavano di 64 MP ultrawide; la scheda ufficiale dice 48 MP. **Non hard-codare nulla: dump da `CameraCharacteristics`.**

## 3. Cosa Google espone all'utente (Pro Controls)

- Pixel 8 Pro è il primo Pixel con **Pro Controls** nativi in Google Camera: focus manuale (con ingrandimento PiP trascinabile), shutter speed, ISO, white balance, brightness ombre, toggle **50 MP full-res** e **RAW+JPEG**.
- HDR+ resta attivo anche con Pro Controls (punto critico per §51: non è RAW puro stile astro-cam).
- Modalità astrofotografia stock: da Night Sight → icona Astro → ON; scatto da **~4 minuti = 16×16 s fusi**. Trigger manuale introdotto con update Camera agosto 2024. In luce forte il tempo si riduce automaticamente.

## 4. Implicazioni per DeepskyEyes

1. Tre camere posteriori + frontale: servono `camera IDs`, `getPhysicalCameraIds()`, `LENS_FACING`, capability logica (§08).
2. OIS su wide+tele, EIS video, LDAF multi-zona, flicker sensor: verificare controllabilità (§05).
3. USB-C 3.2 e Tensor G3 determinano tetto di banda/termica (§09, §10).
4. Pro Controls conferma che **MANUAL_SENSOR esiste almeno sulla main via app stock**, ma va verificato via Camera2 per ogni camera/risoluzione.
5. Il binning 4:1 e la pipeline HDR+ impongono validazione scientifica del RAW (§07).

## Fonti

- https://store.google.com/product/pixel_8_pro_specs?hl=en-US
- https://blog.google/products-and-platforms/devices/pixel/google-pixel-8-pro-camera
- https://blog.google/products-and-platforms/devices/pixel/how-to-use-pixel-8-camera-pro-controls
- https://www.androidauthority.com/google-pixel-8-camera-leak-3333575
- https://www.gsmarena.com/google_pixel_8_pro-review-2629p5.php
- https://www.dpreview.com/reviews/google-pixel-8-pixel-8-pro-review-two-top-smartphones-for-photography
- https://9to5google.com/2023/10/04/pixel-8-pro-camera-controls
- https://support.google.com/pixelcamera/answer/14106982?hl=en
- https://www.androidpolice.com/how-to-manually-turn-on-astrophotography-mode-on-pixel
- https://camerasettings.com/guides/phone/google-pixel-8-pro
