# target platforms
$targets = @(
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-freebsd",
    "aarch64-linux-android",
    "x86_64-linux-android"
)

# cross build
foreach ($target in $targets) {
    Write-Host "==> Building for $target..."
    $result = & cross build --target $target
    Write-Host "✅ Success: $target"
    if ($LASTEXITCODE -ne 0) {
        Write-Error "❌ Build failed for $target"
        exit 1
    }
}
Write-Host "✅ All builds succeeded."
