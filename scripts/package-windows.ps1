$ErrorActionPreference = 'Stop'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --locked --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'Windows release build failed' }
$metadata = cargo metadata --no-deps --format-version 1 --locked | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'bolum').version
$name = "Bolum-$version-windows-x64"
$stage = "target/dist/$name"
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item 'target/x86_64-pc-windows-msvc/release/bolum.exe' $stage
Get-ChildItem 'target/x86_64-pc-windows-msvc/release/*.dll' -ErrorAction SilentlyContinue | Copy-Item -Destination $stage
Copy-Item LICENSE, THIRD_PARTY_NOTICES.txt $stage
@'
Bolum for Windows x64

Extract the entire ZIP before opening bolum.exe. Download a speech model in
Models, focus a text field in another app, and press Control-Alt-Space to start
and stop dictation. The sample button intentionally keeps its result in Bolum.

This build is not code-signed. Windows may show an unknown-publisher warning.
Only approve an app you downloaded from https://github.com/shivrajcode/Bolum/releases.
Microphone access must be enabled for desktop apps in Windows Settings.

Documentation: https://github.com/shivrajcode/Bolum/tree/main/docs
Speech models are downloaded separately; no Python, browser runtime or account
is required. Session history is cleared when Bolum exits.
'@ | Set-Content "$stage/README.txt"
# Exercise the packaged executable from its own directory to catch missing DLLs.
$exe = (Resolve-Path "$stage/bolum.exe").Path
$statusLog = Join-Path ([System.IO.Path]::GetTempPath()) "bolum-status-$PID.log"
$process = Start-Process $exe -ArgumentList 'status' -WorkingDirectory (Resolve-Path $stage).Path -PassThru -Wait -RedirectStandardError $statusLog
if ($process.ExitCode -ne 0 -and $process.ExitCode -ne 1) { throw "Packaged executable failed its startup check: $($process.ExitCode)" }
if ($process.ExitCode -eq 1 -and (Get-Content $statusLog -Raw) -notmatch 'Bolum is not running') { throw 'Unexpected packaged executable response' }
Remove-Item $statusLog
Compress-Archive -Path $stage -DestinationPath "target/dist/$name.zip" -Force
Get-Item "target/dist/$name.zip" | Select-Object Name, Length
$nsis = Join-Path ${env:ProgramFiles(x86)} 'NSIS/makensis.exe'
if (-not (Test-Path $nsis)) { throw 'Install NSIS 3 before packaging the Windows installer' }
$installer = Join-Path (Resolve-Path 'target/dist').Path "Bolum-$version-windows-x64-setup.exe"
& $nsis "/DVERSION=$version" "/DSTAGE=$((Resolve-Path $stage).Path)" "/DOUTFILE=$installer" packaging/windows.nsi
if ($LASTEXITCODE -ne 0) { throw 'NSIS installer generation failed' }
if ($env:CI -eq 'true') {
    $installDir = Join-Path ([System.IO.Path]::GetTempPath()) "Bolum-install-test-$PID"
    $installed = Start-Process $installer -ArgumentList "/S /D=$installDir" -Wait -PassThru
    if ($installed.ExitCode -ne 0 -or -not (Test-Path "$installDir/bolum.exe")) { throw 'Silent installer test failed' }
    if ((Get-ItemProperty 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Uninstall/Bolum').DisplayVersion -ne $version) { throw 'Installed version registration is wrong' }
    $started = Start-Process "$installDir/bolum.exe" -ArgumentList 'status' -Wait -PassThru
    if ($started.ExitCode -ne 1) { throw 'Installed executable startup failed' }
    Start-Process "$installDir/Uninstall.exe" -ArgumentList '/S' -Wait
    for ($i = 0; $i -lt 30 -and (Test-Path "$installDir/bolum.exe"); $i++) { Start-Sleep -Milliseconds 200 }
    if (Test-Path "$installDir/bolum.exe") { throw 'Uninstaller did not remove the installed app' }
    Write-Host 'Windows install, installed startup, and uninstall checks passed'
}
