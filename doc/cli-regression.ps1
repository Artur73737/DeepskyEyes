param([string]$Serial = '44101FDJG003S3', [switch]$Long)
$ErrorActionPreference = 'Stop'
$env:PATH = 'C:\platform-tools;' + $env:PATH
$cliRoot = Split-Path $PSScriptRoot -Parent
$cliExe = Join-Path $cliRoot 'Rust_App\target\release\deepsky-eyes.exe'
$cliOutput = Join-Path $cliRoot ('Rust_App\target\release\captures\CLI_AUDIT_' + (Get-Date -Format 'yyyyMMdd_HHmmss_fff'))
New-Item -ItemType Directory -Path $cliOutput | Out-Null
$cliResults = [System.Collections.Generic.List[object]]::new()
function Invoke-Check([string]$Name, [int]$Expected, [string[]]$Arguments) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $lines = & $cliExe --source adb --serial $Serial @Arguments 2>&1
    $actual = $LASTEXITCODE
    $text = $lines -join "`n"
    $entry = [pscustomobject]@{name=$Name;expected=$Expected;actual=$actual;seconds=$timer.Elapsed.TotalSeconds;arguments=$Arguments;output=$text}
    $cliResults.Add($entry)
    # Generated execution evidence, not hand-maintained documentation.
    $cliResults | ConvertTo-Json -Depth 10 | Out-File -LiteralPath (Join-Path $cliOutput 'results.json') -Encoding utf8
    Write-Output "$Name : exit=$actual expected=$Expected elapsed=$([math]::Round($timer.Elapsed.TotalSeconds,3))s"
    if ($actual -ne $Expected) { throw "$Name failed: $text" }
}
Invoke-Check 'status' 0 @('status')
Invoke-Check 'ping' 0 @('ping')
Invoke-Check 'thermal' 0 @('thermal')
Invoke-Check 'dump' 0 @('capability_dump','--out',(Join-Path $cliOutput 'characteristics.json'))
Invoke-Check 'autofocus' 0 @('autofocus','--camera','0','--out',(Join-Path $cliOutput 'autofocus.json'))
$common = @('capture','--camera','0','--out',$cliOutput,'--exposure-ns','100000000','--sensitivity','100','--wb-preset','daylight')
Invoke-Check 'reject-binned-dng' 1 ($common + @('--project','BAD_DNG','--stream','2032x1536:Dng'))
Invoke-Check 'still-alive-after-rejection' 0 @('status')
Invoke-Check 'reject-size' 1 ($common + @('--project','BAD_SIZE','--stream','9999x9999:Dng'))
Invoke-Check 'reject-unknown-option' 1 ($common + @('--project','BAD_FLAG','--oisx','on'))
Invoke-Check 'reject-processing' 1 ($common + @('--project','BAD_PROCESS','--processing','edge=alien'))
Invoke-Check 'reject-duration' 1 ($common + @('--project','BAD_DURATION','--frame-duration-ns','1'))
Invoke-Check 'reject-crop' 1 ($common + @('--project','BAD_CROP','--crop','0,0,99999,99999'))
Invoke-Check 'reject-kelvin' 1 @('capture','--project','BAD_KELVIN','--out',$cliOutput,'--wb-kelvin','5000')
Invoke-Check 'raw16-binned' 0 ($common + @('--project','RAW16_BIN','--stream','2032x1536:Raw16Le'))
Invoke-Check 'manual-controls' 0 ($common + @('--project','CONTROLS','--frame-duration-ns','200000000','--ois','on','--eis','on','--processing','edge=fast,noise_reduction=fast,hot_pixel=fast,shading=fast,distortion=fast,aberration=off,tonemap=high_quality','--crop','0,0,4080,3072'))
Invoke-Check 'custom-crop' 0 ($common + @('--project','CROP','--crop','1020,768,2040,1536'))
foreach ($wb in @('incandescent','fluorescent','warm_fluorescent','cloudy_daylight')) {
    Invoke-Check "wb-$wb" 0 @('capture','--camera','0','--out',$cliOutput,'--project',"WB_$wb",'--exposure-ns','100000000','--sensitivity','100','--wb-preset',$wb)
}
Invoke-Check 'preview-five' 0 @('preview','--camera','0','--frames','5','--exposure-ns','10000000','--sensitivity','100','--out',(Join-Path $cliOutput 'preview.png'))
Invoke-Check 'jpeg-template' 0 @('request-template','--camera','0','--stream','4032x3024:Jpeg','--out',(Join-Path $cliOutput 'jpeg-request.json'))
Invoke-Check 'jpeg-execute' 0 @('execute','--request',(Join-Path $cliOutput 'jpeg-request.json'),'--out',(Join-Path $cliOutput 'capture.jpg'))
Invoke-Check 'reject-overwrite' 1 @('execute','--request',(Join-Path $cliOutput 'jpeg-request.json'),'--out',(Join-Path $cliOutput 'capture.jpg'))
Invoke-Check 'strict-results-preserve-and-stop' 1 ($common + @('--project','STRICT','--strict-results','--processing','hot_pixel=off,shading=off'))
if ($Long) {
    Invoke-Check 'maximum-exposure' 0 @('capture','--camera','0','--out',$cliOutput,'--project','MAX_EXP','--exposure-ns','16000001084','--sensitivity','100','--wb-preset','daylight')
    Invoke-Check 'sequence-30x1s' 0 @('sequence','--camera','0','--out',$cliOutput,'--project','STABILITY','--frames','30','--exposure-ns','1000000000','--sensitivity','100','--wb-preset','daylight','--delay-ns','0')
}
foreach ($session in Get-ChildItem -LiteralPath $cliOutput -Directory) {
    if (Test-Path -LiteralPath (Join-Path $session.FullName 'session.json')) {
        Invoke-Check "integrity-$($session.Name)" 0 @('session-inspect','--path',$session.FullName)
    }
}
Write-Output "Evidence: $cliOutput"
