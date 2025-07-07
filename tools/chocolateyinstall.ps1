$ErrorActionPreference = 'Stop'

$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$zipPath  = Join-Path $toolsDir 'harboor-sweep.zip'

Get-ChocolateyUnzip -FileFullPath $zipPath -Destination $toolsDir

Copy-Item (Join-Path $toolsDir 'harboor-sweep.exe') `
          (Join-Path $toolsDir 'hs.exe') -Force
