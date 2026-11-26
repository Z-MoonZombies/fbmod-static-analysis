param(
    [Parameter(Mandatory = $true)][string]$PePath,
    [Parameter(Mandatory = $true)][string]$StartRva,
    [Parameter(Mandatory = $true)][string]$EndRva,
    [string]$DumpbinPath
)

function Convert-HexRva([string]$Value) {
    $digits = $Value -replace '^0[xX]', ''
    return [UInt64]::Parse($digits, [Globalization.NumberStyles]::HexNumber)
}

if (-not $DumpbinPath) {
    $installedCommand = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
    if ($installedCommand) {
        $DumpbinPath = $installedCommand.Source
    } else {
        $msvcRoot = Join-Path $env:ProgramFiles 'Microsoft Visual Studio\2022\Community\VC\Tools\MSVC'
        if (Test-Path -LiteralPath $msvcRoot) {
            $versions = Get-ChildItem -LiteralPath $msvcRoot -Directory | Sort-Object Name -Descending
            foreach ($version in $versions) {
                $candidate = Join-Path $version.FullName 'bin\Hostx64\x64\dumpbin.exe'
                if (Test-Path -LiteralPath $candidate) {
                    $DumpbinPath = $candidate
                    break
                }
            }
        }
    }
}

if (-not $DumpbinPath -or -not (Test-Path -LiteralPath $DumpbinPath)) {
    throw 'dumpbin.exe was not found; pass its full path with -DumpbinPath.'
}

$start = [UInt64]0x140000000 + (Convert-HexRva $StartRva)
$end = [UInt64]0x140000000 + (Convert-HexRva $EndRva)
if ($end -le $start) {
    throw 'EndRva must be greater than StartRva.'
}

& $DumpbinPath /DISASM /RAWDATA:NONE $PePath | ForEach-Object {
    if ($_ -match '^\s+([0-9A-F]{16}):') {
        $address = [Convert]::ToUInt64($Matches[1], 16)
        if ($address -ge $start -and $address -lt $end) {
            $_
        }
    }
}
