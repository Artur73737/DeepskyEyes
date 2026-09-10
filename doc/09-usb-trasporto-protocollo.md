# 09 — USB, trasporti e protocollo (ricerca online)

Copre: README §26–33, §84, §85, §98, §102.

## 1. USB Android: host vs accessory

- **Accessory mode:** hardware esterno = host USB, alimenta il bus. **Host mode:** telefono = host, alimenta il bus, enumera device (fotocamere, tastiere...).
- Supporto da API 12+ (backport API 10). Dipende comunque dall'hardware (`<uses-feature>`).
- Classi: `UsbManager`, `UsbDevice/Accessory/Interface/Endpoint/Connection/Request`, `UsbConstants`.
- **AOA 1.0:** control 51 GetProtocol → 52 stringhe (manufacturer/model/description/version/URI/serial, max 256B) → 53 start accessory. PID `0x2D00` accessory, `0x2D01` accessory+adb.
- **AOA 2.0:** + audio (control 58) e HID proxy (register/unregister/send). PID `0x2D02..0x2D05` (audio, audio+adb, accessory+audio, accessory+audio+adb).
- Debug con USB occupata: `adb tcpip 5555` + `adb connect <ip>:5555`.

## 2. Strategia DeepskyEyes (§27/§85/§98)

- Sviluppo: `PC → ADB → socket/service Android`.
- Produzione candidati: ADB, Open Accessory, USB accessory/host, seriale-like se HW lo consente, TCP fallback.
- ADB non deve diventare dipendenza architetturale. Protocollo **transport-independent**:
  `Camera code → Protocol → Transport (ADB/USB/TCP/Mock)`.
- Da misurare (§98): ricarica durante camera attiva, banda/sostenuta, stabilità cavi, modo USB, comportamento ADB/accessory, reconnect, transport di produzione.

## 3. Design protocollo (§29–33)

- Versioned, framed, length-delimited, checksummed, request/response + eventi, estensibile.
- Header logico: `protocol_version, message_type, flags, request_id, payload_length, sequence_number` + payload + checksum.
- `request_id` per correlare comando/risposta (es. SET_EXPOSURE 912 → OK requested/applied).
- Famiglie comandi: SYSTEM (HELLO/VERSION/STATUS/PING/TIME), DEVICE (INFO/CAMERAS/CAPABILITIES),
  CAMERA (OPEN/CLOSE/CONFIGURE/STATE), CONTROL (exposure/sensitivity/frame/focus/AF/AWB/gains/zoom/crop/processing),
  PREVIEW, CAPTURE, SEQUENCE, STORAGE, DIAGNOSTICS (LOG/METRICS/THERMAL/RESULT).
- Eventi: CONNECTED/DISCONNECTED, OPENED/CLOSED/ERROR, STARTED/COMPLETED/FAILED, FRAME_*,
  TEMPERATURE/THERMAL_WARNING, SEQUENCE_*.
- Errori machine-readable: UNSUPPORTED_PARAMETER, INVALID_VALUE, VALUE_CLAMPED, CAMERA_BUSY/DISCONNECTED,
  SESSION_CONFIGURATION_FAILED, CAPTURE_FAILED, RAW/FORMAT/SIZE_NOT_SUPPORTED, TRANSPORT/TIMEOUT/THERMAL/STORAGE/PROTOCOL_ERROR + testo human supplementare.
- Clock: due orologi (Android/PC) non sincronizzati → PING/timestamp/offset; futuro UTC/GPS/NTP.

## 4. Sicurezza (§84) e framing (§102)

Non accettare comandi da peer arbitrari; definire handshake/sessione/versione/identità device.
Non esporre endpoint LAN di produzione senza auth. Checklist §102: framing, serializzazione,
IDs, checksum, versioning, compatibilità, reconnect/resume, auth, indipendenza transport.

## Fonti

- https://developer.android.com/develop/connectivity/usb
- https://developer.android.com/develop/connectivity/usb/accessory
- https://developer.android.com/develop/connectivity/usb/host
- https://source.android.com/docs/core/interaction/accessories/aoa
- https://source.android.com/docs/core/interaction/accessories/aoa2
- https://developer.android.com/reference/android/hardware/usb/package-summary
- https://source.android.com/docs/core/camera/multi-camera (stream rules utili al framing)
