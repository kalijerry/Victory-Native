# Starts the title through play.ps1 with the settings for one PC: NVIDIA RTX 5080 (16 GB),
# AMD Ryzen 9 9950X3D (16 cores, 32 threads, 3D V-Cache on one of its two CCDs), 32 GB DDR5.
# Why each setting is here, and what to set in Windows first: docs/pc-rtx5080-9950x3d.md.
#   scripts\play-rtx5080.ps1 -Exe <path of your own copy's executable>
# -NoPatches   without renderer patches 1 and 2 (VICTORY_PORT_CONSTANTS, VICTORY_PORT_DESCRIPTORS)
# -VCacheCcd   the whole process on logical processors 0-15 only (the CCD with the 3D V-Cache)
# -Safe, -RayTracing, -Build, -More   passed on to play.ps1 (-Safe also leaves the patches out)
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$NoPatches,
    [switch]$VCacheCcd,
    [switch]$Safe,
    [switch]$RayTracing,
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [hashtable]$More = @{}
)
$settings = @{}
# The README's figures (about 53 ms a frame) are with both patches on. Each falls back to the
# old way where it cannot apply: patch 1 per region of memory, patch 2 by a check every frame.
if (-not $NoPatches -and -not $Safe) {
    $settings.VICTORY_PORT_CONSTANTS = '1'
    $settings.VICTORY_PORT_DESCRIPTORS = '1'
}
# What is asked with -More comes last and wins (-More @{VICTORY_PORT_CONSTANTS='0'}).
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }

# A child process gets its parent's affinity, and the title runs inside victory.exe's process.
# The layer pins nothing itself on this CPU: its fast-core pinning (crates/victory-gpu/src/cores.rs)
# is for processors with two efficiency classes, and every core of a 9950X3D has the same one.
$self = Get-Process -Id $PID
$before = $self.ProcessorAffinity
$pinned = $false
if ($VCacheCcd) {
    $count = [Environment]::ProcessorCount
    if ($count -ne 32) {
        Write-Host "-VCacheCcd is for a 9950X3D with SMT on (32 logical processors); this PC has $count. Affinity left as it is."
    } else {
        $self.ProcessorAffinity = [IntPtr]0xFFFF
        $pinned = $true
        Write-Host 'Logical processors 0-15 only (CCD 0, the one with the 3D V-Cache on a 9950X3D).'
    }
}
try {
    & (Join-Path $PSScriptRoot 'play.ps1') -Exe $Exe -Safe:$Safe -RayTracing:$RayTracing -Build $Build -More $settings
} finally {
    if ($pinned) { $self.ProcessorAffinity = $before }
}
