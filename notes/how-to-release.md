```nushell

# update version in Cargo.toml
cargo b # to update Cargo.lock

let PREVIOUS_RELEASE = git tag | lines | where {$in | str starts-with "v"} | sort | last
let RELEASE = open Cargo.toml  | get package.version

# grab the raw commit messages between the previous release and now
# create the release notes
git log --format=%s $"($PREVIOUS_RELEASE)..HEAD" | vipe | save -f $"changes/($RELEASE).md"
git add changes

git commit -a -m $"chore: release ($RELEASE)"
git push

cargo publish
cargo install cross-stream --locked

rm ~/bin/xs
brew uninstall cross-stream
which xs # should be /Users/andy/.cargo/bin/xs
# test the new version

let pkgdir = $"cross-stream-($RELEASE)"
let tarball = $"cross-stream-($RELEASE)-macos.tar.gz"

mkdir $pkgdir
cp /Users/andy/.cargo/bin/xs $pkgdir
tar -czvf $tarball -C $pkgdir xs 

# git tag $"v($RELEASE)"
# git push --tags
# ^^ not needed, as the next line will create the tags -->
gh release create $"v($RELEASE)" -F $"changes/($RELEASE).md" $tarball

shasum -a 256 $tarball

# update: git@github.com:cablehead/homebrew-tap.git

brew install cablehead/tap/cross-stream
which xs # should be /opt/homebrew/bin/xs
# test the new version
```
