# 13 — Calibrazione e workflow astrofotografico (ricerca online)

Copre: README §53, §54, §95, §109–112.

## 1. Frame di calibrazione

- **LIGHT:** cielo, segnale + rumore.
- **DARK:** stesso tempo/gain/offset/**temperatura** dei light, otturatore chiuso → mappa dark current + bias. Master dark = mediana/media di N dark.
- **FLAT:** sorgente uniforme, ~1/3–1/2 dinamica → vignettatura/polvere/sensitività pixel. Si calibra con bias (flat brevi <~30 s) o dark-flat (flat lunghi), mai bias di camera malevola.
- **BIAS/OFFSET:** esposizione minima a buio → read noise + offset. Su alcune CMOS i bias cortissimi sono inaffidabili → usare flat-dark.
- Regola (§53): dark `ISO800/15s/30°C` **non** applicabile a `ISO1600/15s/15°C`. Ogni master registra exposure/gain/temp/focus/resolution/orientation/camera.

## 2. Teoria rumore (§95)

Read noise (elettronica, indipendente da t), dark current (elettroni termici, ∝ t e T),
FPN, hot pixel, amp glow, linearità sensore/gain, saturazione, DR, ISO invariance, dipendenza termica.
Moderno CMOS (post-2008) ha dark suppression on-sensor; converter con lens profile applica flat + bad-pixel da raw.
Per smartphone verificare quanto resta valido.

## 3. Dark library (§54)

Matrice futura ISO {100,200,400,800,1600} × exposure {1,5,10,15,30 s} × T {10..30 °C}, generata empiricamente.
Utile per 300×15 s senza riscattare dark ogni notte (se T matchata).

## 4. Futuro (non MVP)

- Plate solving: `preview → star detection → solver → RA/Dec → mount`.
- Mount: INDI/ASCOM/Alpaca (solo predisposizione).
- Guiding: centroide → errore → correzione (non MVP).
- Tipi sessione iniziali: LIGHT/DARK/FLAT/BIAS/TEST; futuri FOCUS/PLATE_SOLVE/LIVE_STACK.
- Output futuri: FITS, Siril/PixInsight workflow.

## Fonti

- https://stackingstarlight.com/calibration
- https://www.astroworldcreations.com/blog/understanding-flats-part3-conclusions
- https://theastromanual.com/image-stacking-processing-raw-frames-to-astrophotos
- https://www.opticalmechanics.com/mastering-calibration-frames-for-deep-sky-astrophotography
- https://www.aavso.org/bias-frames-and-cmos-cameras-scaled-and-unscaled-darks
- https://stellarnomads.com/calibration-frames
- http://astrosurf.com/jwisn/basics.htm
- https://clarkvision.com/articles/astrophotography.image.processing
