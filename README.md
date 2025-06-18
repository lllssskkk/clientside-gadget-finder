# ghunter4chrome

This is a port of [GHunter](https://github.com/KTH-LangSec/ghunter) to
detect client-side gadgets in webpages using Chromium.

## Running

> [!TIP]
> Compiling Chromium can take a few hours.
> For that reason, pre-compiled binaries are provided for certain revisions
> of this repository.
> See ["Using pre-built version of Chromium"](#using-pre-built-version-of-chromium)
> for more information.

Running this repository requires Nix to be installed in the target system.
An easy way to do that on any Linux distro is through the
[Determinate Systems Nix Installer](https://github.com/DeterminateSystems/nix-installer).
If you don't have root access, you can instead
[install Nix in your user only](https://zameermanji.com/blog/2023/3/26/using-nix-without-root/),
which should be enough for this project (might need workarounds for certain commands below, though).

> [!NOTE]
> The commands below assume that the `nix-command` and `flakes` experimental features are enabled,
> which can be done by adding the following to your nix config (see `man 5 nix.conf`):
>
> ```
> experimental-features = nix-command flakes
> ```

This project has been tested with Nix 2.18.10 and 2.24.10, but should work with newer Nix versions as well.

> [!CAUTION]
> This chromium build is always run with `--no-sandbox`, which significantly decreases the security
> of the browser.
> Avoid running on a system with sensitive information.

The flake exports a `chromium-ghunter` package.

- To open a shell with this binary in `$PATH`, run `nix shell .#chromium-ghunter`.
  The custom Chromium can now be run with `chromium-ghunter`.
- To run this binary directly, run `nix run .#chromium-ghunter -- [args]`.
- To build the package and place a symlink in the current directory,
  run `nix build .#chromium-ghunter`.
  The custom Chromium can now be run with `./result/bin/chromium-ghunter`.

## Using pre-built version of Chromium

To avoid compiling Chromium (which can require reasonably powered hardware and time), an archive
containing a Nix binary cache (which has the resulting closure of Chromium) can be found in
the [releases page of this repository](https://github.com/diogotcorreia/ghunter4chrome/releases).

Make sure you have checked out this repo in the **same revision as the release you are downloading
from**, otherwise the derivation might not match and still cause a rebuild.
If you want to make sure it is the same, run `nix eval .#chromium-ghunter` and check it against
the derivation path in the release.

<details>
<summary>How to generate this archive</summary>

If you are editing the provided chromium derivation and want to regenerate this archive
for distribution, you can take the following steps:

```sh
# build the derivation
nix build .#chromium-ghunter

# copy the result to the binary cache at /tmp/nix-cache (file:// is important!)
nix copy --to file:///tmp/nix-cache ./result

# create tar with the /tmp/nix-cache directory
pushd /tmp
tar cvf nix-cache.tar nix-cache
rm -rf /tmp/nix-cache # optional
popd
```

</details>

After downloading the `nix-cache.tar` archive, it can be extracted and imported with the following commands:

```sh
# extract tar
tar xvf nix-cache.tar

# copy chromium from the cache
# NOTE: run with sudo if your user is not in Nix's trusted-users
nix copy --all --from "file://$(pwd)/nix-cache" --no-check-sigs

# if command above fails, it's due to a bug in nix:
# https://github.com/NixOS/nix/issues/8473
# workaround (needs to be run inside this flake):
nix copy --from "file://$(pwd)/nix-cache" --no-check-sigs "$(nix eval --raw --read-only .#chromium-ghunter)"
```

The custom version of Chromium can now be run according to ["Running"](#running).

## Developing

When changing the code of this chromium build, it is useful to only preform incremental
compilation instead of rebuilding from scratch every time.
We can take advantage of `nix develop` to achieve this, which opens a shell dedicated
to building the package.

```sh
nix develop .#chromium-ghunter-unwrapped
# add options like `--cores 26` to limit the number of cores used for the build
```

While inside the shell, use

```sh
eval "${unpackPhase:-unpackPhase}"
cd "$sourceRoot"
eval "${patchPhase:-patchPhase}"
eval "${configurePhase:-configurePhase}" # run configurePhase again if you exit the shell
eval "${buildPhase:-buildPhase}" # does the actual build
```

## Sinks

Below is a comprehensive list of all the sinks detected by this
project (sorted alphabetically):

- [`Attr#value`](https://developer.mozilla.org/en-US/docs/Web/API/Attr/value) (setter)
- [`CSSStyleDeclaration#cssText`](https://developer.mozilla.org/en-US/docs/Web/API/CSSStyleDeclaration/cssText) (setter)
- [`Document#cookie`](https://developer.mozilla.org/en-US/docs/Web/API/Document/cookie) (setter)
- [`Document#createAttribute()`](https://developer.mozilla.org/en-US/docs/Web/API/Document/createAttribute)
- [`Document#createAttributeNS()`](https://developer.mozilla.org/en-US/docs/Web/API/Document/createAttributeNS) (qualified name)
- [`Document#location`](https://developer.mozilla.org/en-US/docs/Web/API/Document/location) (setter)
- [`Document#write()`](https://developer.mozilla.org/en-US/docs/Web/API/Document/write)
- [`Document#writeln()`](https://developer.mozilla.org/en-US/docs/Web/API/Document/writeln)
- [`Element#innerHTML`](https://developer.mozilla.org/en-US/docs/Web/API/Element/innerHTML) (setter)
- [`Element#insertAdjacentHTML()`](https://developer.mozilla.org/en-US/docs/Web/API/Element/insertAdjacentHTML)
- [`Element#outerHTML`](https://developer.mozilla.org/en-US/docs/Web/API/Element/outerHTML) (setter)
- [`Element#setAttribute()`](https://developer.mozilla.org/en-US/docs/Web/API/Element/setAttribute) (key and value)
- [`Element#setHTMLUnsafe()`](https://developer.mozilla.org/en-US/docs/Web/API/Element/setHTMLUnsafe)
- [`eval()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/eval)
- [`Function()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Function/Function) (constructor)
- [`HTMLAnchorElement#href`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLAnchorElement/href) (setter)
- [`HTMLElement#style`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/style) (setter)
- [`HTMLEmbedElement#src`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLEmbedElement/src) (setter)
- [`HTMLIFrameElement#src`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLIFrameElement/src) (setter)
- [`HTMLImageElement#src`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/src) (setter)
- [`HTMLImageElement#srcset`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/srcset) (setter)
- [`HTMLScriptElement#src`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLScriptElement/src) (setter)
- [`HTMLScriptElement#text`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLScriptElement/text) (setter)
- [`Location#assign()`](https://developer.mozilla.org/en-US/docs/Web/API/Location/assign)
- [`Location#hash`](https://developer.mozilla.org/en-US/docs/Web/API/Location/hash) (setter)
- [`Location#host`](https://developer.mozilla.org/en-US/docs/Web/API/Location/host) (setter)
- [`Location#hostname`](https://developer.mozilla.org/en-US/docs/Web/API/Location/hostname) (setter)
- [`Location#href`](https://developer.mozilla.org/en-US/docs/Web/API/Location/href) (setter)
- [`Location#pathname`](https://developer.mozilla.org/en-US/docs/Web/API/Location/pathname) (setter)
- [`Location#port`](https://developer.mozilla.org/en-US/docs/Web/API/Location/port) (setter)
- [`Location#protocol`](https://developer.mozilla.org/en-US/docs/Web/API/Location/protocol) (setter)
- [`Location#replace()`](https://developer.mozilla.org/en-US/docs/Web/API/Location/replace)
- [`Location#search`](https://developer.mozilla.org/en-US/docs/Web/API/Location/search) (setter)
- [`Window#location`](https://developer.mozilla.org/en-US/docs/Web/API/Window/location) (setter)

Additionally, during object assignment, the key is considered a sink (e.g., `obj[tainted] = "foo"` is detected).
