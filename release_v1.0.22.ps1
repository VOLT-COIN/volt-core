$releaseName = "v1.0.22"
$zipFile = "Volt_App_v1.0.22.zip"
$sourceZip = "volt-windows-binaries.zip"

# Rename if exists
if (Test-Path $sourceZip) {
    Rename-Item -Path $sourceZip -NewName $zipFile -Force
    Write-Host "Renamed $sourceZip to $zipFile"
}

# Create Release Notes
$notes = @"
## 🚀 Critical Update: Connectivity & Stability

### 🔌 P2P Fixes
- **Instant Discovery**: Removed 60s startup delay.
- **Always-On Bootstrap**: Force-add official bootstrap server.
- **DNS Fallback**: Added direct IP backup.

### 🧹 Repository Cleanliness
- **Pure Source**: Clean Core + Wallet structure.
- **CI/CD**: Automated builds via GitHub Actions.

### 📦 Download
- Windows (64-bit): $zipFile
"@

Write-Host "Creating GitHub Release $releaseName..."
gh release create $releaseName $zipFile --title "Volt v1.0.22: Instant P2P & Clean Build" --notes "$notes"

Write-Host "Release Published Successfully!"
