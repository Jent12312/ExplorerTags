Stop-Process -Name dllhost, explorer, explorer-tags-tray, ExplorerTagsUI -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

$appDir = "$env:APPDATA\ExplorerTags"
if (-not (Test-Path $appDir)) {
    New-Item -ItemType Directory -Path $appDir -Force | Out-Null
}

Copy-Item "B:\ExplorerTags\target\release\ExplorerTagsUI.exe" "$appDir\ExplorerTagsUI.exe" -Force
Copy-Item "B:\ExplorerTags\target\release\ExplorerTags.dll" "$appDir\ExplorerTags.dll" -Force
Copy-Item "B:\ExplorerTags\target\release\explorer-tags-tray.exe" "$appDir\explorer-tags-tray.exe" -Force
Copy-Item "B:\ExplorerTags\res\PropertySchema.propdesc" "$appDir\PropertySchema.propdesc" -Force

regsvr32.exe /s "$appDir\ExplorerTags.dll"
Start-Process explorer.exe
Start-Process "$appDir\explorer-tags-tray.exe"
Write-Output "DEPLOY_SUCCESSFUL"
