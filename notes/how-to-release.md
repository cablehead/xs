```nushell
# update version in Cargo.toml
let PREVIOUS_RELEASE = "0.1.0"
$env.RELEASE = open Cargo.toml  | get package.version

# grab the raw commit messages between the previous release and now
# create the release notes
git log --format=%s $"v($PREVIOUS_RELEASE)..HEAD" | vipe | save $"changes/($env.RELEASE).md"

git commit -a -m $"chore: release ($env.RELEASE)"
git push

cargo publish
cargo install cross-stream --locked

rm ~/bin/xs
brew uninstall cross-stream
which xs # should be /Users/andy/.cargo/bin/xs
# test the new version

let pkgdir = $"cross-stream-($env.RELEASE)"
let tarball = $"cross-stream-($env.RELEASE)-macos.tar.gz"

mkdir $pkgdir
cp /Users/andy/.cargo/bin/xs $pkgdir
tar -czvf $tarball -C $pkgdir xs 

# git tag $"v($env.RELEASE)"
# git push --tags
# ^^ not needed, as the next line will create the tags -->
gh release create $"v($env.RELEASE)" -F $"changes/($env.RELEASE).md" $tarball

shasum -a 256 $tarball

# update: git@github.com:cablehead/homebrew-tap.git

brew install cablehead/tap/cross-stream
which xs # should be /opt/homebrew/bin/xs
# test the new version
```
