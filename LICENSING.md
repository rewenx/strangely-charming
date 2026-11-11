# Licensing and credits

Strangely Charming contains original work and third-party material under different licenses. The MIT license does not replace the terms for the separately identified artwork below. Keep the relevant copyright, attribution and license notices when copying or adapting any part of the project.

## 1. Original Strangely Charming work: MIT

The mod's original code, documentation and original artwork by EvanderFox and Strangely Charming contributors are licensed under the [MIT License](LICENSE). This is the project's general license, subject to the third-party exceptions in this file. Modifications to third-party material remain subject to that material's applicable terms.

## 2. Nexo Assets: MCModels terms

Models and artwork by **Six / Nexo Assets** were purchased in these two packs:

- [**Amethyst Blocks**](https://mcmodels.net/products/14702/amethyst-blocks): the eight decorative amethyst finishes and the base artwork used by the carved variants. The additional carvings are by EvanderFox.
- [**Tombstone Furniture**](https://mcmodels.net/products/12671/tombstones): the Short, Ornate and Tall Tombstone models and textures.

These assets are subject to the [MCModels terms](https://mcmodels.net/information/terms), identified in this document as **LicenseRef-MCModels**, not MIT. The [Nexo Assets FAQ](https://mcmodels.net/vendors/18/nexo-assets) permits use in non-Minecraft projects and asks for credit where possible.

The marketplace's Purchases section gives a “license to use commercially in-game.” Its general license allows selling cosmetics or rewards on a platform the licensee operates, but expressly excludes “Re-sell the assets; whether they are bundled with other files or not.” It also restricts source sharing with unlicensed people and redistribution without permission.

The reviewed public terms and FAQ do not expressly authorize redistribution in downloadable mod bundles or editable-source form. This notice records provenance and applicable terms; it is not an additional redistribution grant or a claim that every distribution method is cleared. Obtain specific permission where required. Public terms checked November 11, 2026.

The public source checkout omits the standalone Nexo runtime files listed in [tools/release_art.json](tools/release_art.json). The build helper can recover those bytes from the existing release archive. Embedding or recovering artwork this way does not change its license or grant permission to redistribute it.

## 3. Charm, Strange, Horizon and the native template: MIT

**Sven Hjól and contributors** created [Charm](https://github.com/svenhjol/CharmLegacy) and [Strange](https://github.com/svenhjol/Strange). Derived artwork, recipes, structures and code retain their MIT notices. **mcvinnyq** is credited for the Writing Desk artwork.

- [Charm and Strange copyright and license notices](LICENSE.charm-and-strange)
- [Charm notice](mixins/strangely_charming/LICENSE.charm)
- [Horizon notice](mixins/strangely_charming/LICENSE.horizon)
- [Horizon native template / Veinminer contributors notice](mixins/strangely_charming/LICENSE.template)

## 4. Runebreaker artwork: CC BY 4.0

The Runebreaker artwork originated with **Malcolm Riley's** [diamond hammer sprite](https://github.com/malcolmriley/unused-textures/blob/8b85ad732023174d9a9da74e6bf2272d6c0bef17/items/tool_fancy_diamond_hammer.png), under [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/). The current sprite includes updated artwork supplied by EvanderFox; it is not the unmodified original PNG. Attribution and CC BY 4.0 terms are retained for the Riley-derived artwork.

## 5. Protogen model: CC BY-NC-SA 4.0; Protomod: MIT

**Endles13** created the [Minecraft Protogen Model](https://sketchfab.com/3d-models/minecraft-protogen-model-1cf00d1d4fda4a008036410fc213bdee), adapted by **downloadableduck** for [Protomod](https://modrinth.com/mod/protomod). The model and this port's model adaptations are covered by [CC BY-NC-SA 4.0](mixins/strangely_charming/LICENSE.protogen-model), including its noncommercial and share-alike requirements. This is an exception to the project's MIT license.

[Protomod's MIT notice](mixins/strangely_charming/LICENSE.protomod) is also preserved for its contributions. The port converts geometry, pivots, UVs and animation keyframes for Horizon. Protomod supplies the Ice skin; EvanderFox supplied six recolors. See the [component notice](mixins/strangely_charming/NOTICE) for the pinned source and adaptation details. Protomod's MIT notice does not replace the underlying model's CC terms.

## 6. Other credits

Ears-compatible skin formats and geometry are credited to **exa/unascribed and Ears contributors**. Feature inspiration includes **Vazkii / Violet Moon's Quark** and **henkelmax's Easy Villagers**. The [component notice](mixins/strangely_charming/NOTICE) preserves these credits and the distinction between adaptations, original implementations, and referenced game resources. Strangely Charming is unofficial. AI assistance was used in creating the mod and its branding.

## SPDX expression

For the combined mod, including the separately licensed artwork:

```text
MIT AND CC-BY-4.0 AND CC-BY-NC-SA-4.0 AND LicenseRef-MCModels
```

`AND` describes the different terms applicable to the included components; it does not put each file under all four licenses. `LicenseRef-MCModels` is this project's custom reference to the MCModels terms identified in section 2, not an SPDX-listed standard license. This expression identifies terms, not distribution permission. A submission form or reviewer may require the linked terms and additional written permission; verify its current acceptance requirements. It is not sufficient to describe the complete mod as MIT alone.

## Distribution documents

New archives built by `tools/package.py` include the MIT license, this overview and all existing component notices under `mixins/strangely_charming/`. Horizon requires recognized notice filenames there, so the archive calls this overview `LICENSES.md` and adjusts its relative links. The repository's canonical overview remains `LICENSING.md`. Previously published archives retain their original bytes and accompanying notices; rebuilding with these documentation changes produces a different archive and must not replace an existing signed release in place.
