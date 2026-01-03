$ZipFile = "Volt_App_v1.0.21.zip"
$Version = "v1.0.21"
$Notes = "Release v1.0.21: Wallet Launcher, Auto-Auth, and Web Wallet RPC Fixes. Fully compatible with Miners (Public Broadcast)."

# 1. Commit Zip to Repo (Backup)
git add $ZipFile
git commit -m "Release $Version Artifact"
git push origin main

# 2. GitHub Release (Try GH CLI)
if (Get-Command gh -ErrorAction SilentlyContinue) {
    Write-Host "Creating GitHub Release..."
    gh release create $Version --title "$Version" --notes "$Notes" --target main
    gh release upload $Version $ZipFile --clobber
} else {
    Write-Warning "GitHub CLI (gh) not found. ZIP pushed to repo files only."
}
