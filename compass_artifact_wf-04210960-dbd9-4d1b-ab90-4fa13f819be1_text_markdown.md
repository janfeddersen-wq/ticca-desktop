# Official hex color values for popular editor themes

All color values below are sourced from official theme documentation, GitHub repositories, and authoritative specifications. Each theme's palette has been mapped to standardized UI color tokens for backgrounds, text, borders, accents, and status indicators.

## Dark themes at a glance

For quick implementation, here are the core dark theme palettes side-by-side:

| Token | Dracula | Nord | Catppuccin Mocha | Tokyo Night | One Dark | Gruvbox Dark | Solarized Dark |
|-------|---------|------|------------------|-------------|----------|--------------|----------------|
| **bg_base** | `#191A21` | `#2E3440` | `#1e1e2e` | `#1a1b26` | `#282c34` | `#282828` | `#002b36` |
| **bg_surface** | `#282A36` | `#3B4252` | `#181825` | `#16161e` | `#21252b` | `#3c3836` | `#073642` |
| **bg_elevated** | `#343746` | `#434C5E` | `#313244` | `#292e42` | `#2c323c` | `#504945` | `#586e75` |
| **bg_hover** | `#44475A` | `#4C566A` | `#45475a` | `#283457` | `#3e4452` | `#665c54` | `#073642` |
| **text_primary** | `#F8F8F2` | `#ECEFF4` | `#cdd6f4` | `#c0caf5` | `#abb2bf` | `#ebdbb2` | `#839496` |
| **text_secondary** | `#6272A4` | `#D8DEE9` | `#bac2de` | `#a9b1d6` | `#828997` | `#d5c4a1` | `#93a1a1` |
| **text_muted** | `#6272A4` | `#4C566A` | `#a6adc8` | `#565f89` | `#5c6370` | `#bdae93` | `#586e75` |
| **border_subtle** | `#44475A` | `#3B4252` | `#313244` | `#15161e` | `#3b4048` | `#504945` | `#073642` |
| **border_default** | `#6272A4` | `#4C566A` | `#6c7086` | `#3b4261` | `#4b5263` | `#7c6f64` | `#586e75` |
| **accent** | `#BD93F9` | `#88C0D0` | `#89b4fa` | `#7aa2f7` | `#61afef` | `#83a598` | `#268bd2` |
| **accent_hover** | `#D6ACFF` | `#8FBCBB` | `#89dceb` | `#3d59a1` | `#528bff` | `#8ec07c` | `#6c71c4` |
| **accent_muted** | `#815CD6` | `#5E81AC` | `#b4befe` | `#394b70` | `#3a4a5e` | `#458588` | `#2aa198` |
| **success** | `#50FA7B` | `#A3BE8C` | `#a6e3a1` | `#9ece6a` | `#98c379` | `#b8bb26` | `#859900` |
| **warning** | `#FFB86C` | `#EBCB8B` | `#f9e2af` | `#e0af68` | `#e5c07b` | `#fabd2f` | `#b58900` |
| **danger** | `#FF5555` | `#BF616A` | `#f38ba8` | `#db4b4b` | `#e06c75` | `#fb4934` | `#dc322f` |

## Light themes at a glance

| Token | Catppuccin Latte | Gruvbox Light | Solarized Light |
|-------|------------------|---------------|-----------------|
| **bg_base** | `#eff1f5` | `#fbf1c7` | `#fdf6e3` |
| **bg_surface** | `#e6e9ef` | `#ebdbb2` | `#eee8d5` |
| **bg_elevated** | `#ccd0da` | `#d5c4a1` | `#93a1a1` |
| **bg_hover** | `#bcc0cc` | `#bdae93` | `#eee8d5` |
| **text_primary** | `#4c4f69` | `#3c3836` | `#657b83` |
| **text_secondary** | `#5c5f77` | `#504945` | `#586e75` |
| **text_muted** | `#6c6f85` | `#665c54` | `#93a1a1` |
| **border_subtle** | `#ccd0da` | `#d5c4a1` | `#eee8d5` |
| **border_default** | `#9ca0b0` | `#a89984` | `#93a1a1` |
| **accent** | `#1e66f5` | `#076678` | `#268bd2` |
| **accent_hover** | `#04a5e5` | `#458588` | `#6c71c4` |
| **accent_muted** | `#7287fd` | `#83a598` | `#2aa198` |
| **success** | `#40a02b` | `#79740e` | `#859900` |
| **warning** | `#df8e1d` | `#b57614` | `#b58900` |
| **danger** | `#d20f39` | `#9d0006` | `#dc322f` |

---

## Dracula theme

The Dracula palette originates from draculatheme.com and uses a distinctive purple-heavy aesthetic with high-contrast syntax colors.

### Complete token mapping

**Background layers** establish the visual hierarchy with four distinct shades. The darkest value `#191A21` (Background Darker) serves as bg_base, while the standard `#282A36` (Background) works for panels and cards. Elevated surfaces use `#343746` (Background Light), and `#44475A` (Selection) provides the hover state.

**Text colors** use `#F8F8F2` (Foreground) for primary content. Dracula uniquely uses the same `#6272A4` (Comment) color for both secondary and muted text states, creating a distinctive two-tier text hierarchy.

**Border colors** map to `#44475A` for subtle separators and `#6272A4` for visible borders.

**Accent colors** center on Dracula's signature purple `#BD93F9`, with `#D6ACFF` (AnsiBrightBlue) for hover states and `#815CD6` (Functional Purple) for muted backgrounds like message bubbles.

**Status colors** are highly saturated: green `#50FA7B`, orange `#FFB86C`, and red `#FF5555`. For UI-only contexts requiring less intensity, Dracula provides functional variants—`#089108` (green), `#A39514` (orange), and `#DE5735` (red).

### Additional syntax colors
| Color | Hex | Use |
|-------|-----|-----|
| Pink | `#FF79C6` | Keywords |
| Cyan | `#8BE9FD` | Support functions |
| Yellow | `#F1FA8C` | Functions |

---

## Nord theme

Nord's palette from nordtheme.com draws inspiration from arctic landscapes, organized into four distinct groups: Polar Night (backgrounds), Snow Storm (text), Frost (accents), and Aurora (status).

### Complete token mapping

**Background layers** progress through the Polar Night palette: `#2E3440` (nord0) as the origin base, `#3B4252` (nord1) for panels and sidebars, `#434C5E` (nord2) for active lines and inputs, and `#4C566A` (nord3) for hover states.

**Text colors** use Snow Storm values with `#ECEFF4` (nord6) for primary text, `#D8DEE9` (nord4) for secondary content, and `#4C566A` (nord3) pulled from Polar Night for truly muted/disabled states.

**Borders** use `#3B4252` (nord1) for subtle separators and `#4C566A` (nord3) for visible borders.

**Accent colors** draw from the Frost palette—**nord8** `#88C0D0` serves as the bright primary accent, **nord7** `#8FBCBB` provides a calmer hover alternative, and **nord10** `#5E81AC` works as a muted background accent. Nord9 `#81A1C1` is also commonly used for keywords.

**Status colors** come from Aurora: green `#A3BE8C` (nord14), warning yellow `#EBCB8B` (nord13), and error red `#BF616A` (nord11). Additional Aurora colors include orange `#D08770` (nord12) and purple `#B48EAD` (nord15).

---

## Catppuccin Mocha (dark variant)

Catppuccin's Mocha flavor from catppuccin.com provides 26 precisely defined colors with clear semantic naming, making it exceptionally well-suited for design systems.

### Complete token mapping

**Background layers** use Base `#1e1e2e` as the primary background, Mantle `#181825` for secondary panes and cards, Surface 0 `#313244` for elevated elements, and Surface 1 `#45475a` for hover states. The deepest shade, Crust `#11111b`, is available for maximum depth.

**Text colors** follow clear hierarchy: Text `#cdd6f4` for primary content, Subtext 1 `#bac2de` for labels and sub-headlines, and Subtext 0 `#a6adc8` for placeholders. Overlay colors (0-2) provide additional muted options.

**Borders** use Surface 0 `#313244` for subtle separators and Overlay 0 `#6c7086` for visible borders, per the official style guide.

**Accent colors** default to Blue `#89b4fa` for primary actions, Sky `#89dceb` for hover states, and Lavender `#b4befe` for softer accent backgrounds. Mauve `#cba6f7` is a popular alternative primary accent.

**Status colors** include Green `#a6e3a1` for success, Yellow `#f9e2af` for warnings (with Peach `#fab387` as an alternative), and Red `#f38ba8` for errors.

### Full Mocha palette reference
| Color | Hex | Color | Hex |
|-------|-----|-------|-----|
| Rosewater | `#f5e0dc` | Text | `#cdd6f4` |
| Flamingo | `#f2cdcd` | Subtext 1 | `#bac2de` |
| Pink | `#f5c2e7` | Subtext 0 | `#a6adc8` |
| Mauve | `#cba6f7` | Overlay 2 | `#9399b2` |
| Red | `#f38ba8` | Overlay 1 | `#7f849c` |
| Maroon | `#eba0ac` | Overlay 0 | `#6c7086` |
| Peach | `#fab387` | Surface 2 | `#585b70` |
| Yellow | `#f9e2af` | Surface 1 | `#45475a` |
| Green | `#a6e3a1` | Surface 0 | `#313244` |
| Teal | `#94e2d5` | Base | `#1e1e2e` |
| Sky | `#89dceb` | Mantle | `#181825` |
| Sapphire | `#74c7ec` | Crust | `#11111b` |
| Blue | `#89b4fa` | | |
| Lavender | `#b4befe` | | |

---

## Catppuccin Latte (light variant)

Latte inverts Catppuccin's structure for light mode while maintaining the same 26-color semantic system.

### Complete token mapping

**Background layers** flip to light tones: Base `#eff1f5`, Mantle `#e6e9ef`, Surface 0 `#ccd0da`, and Surface 1 `#bcc0cc` for hover states.

**Text colors** become darker: Text `#4c4f69`, Subtext 1 `#5c5f77`, and Subtext 0 `#6c6f85`.

**Borders** use Surface 0 `#ccd0da` for subtle separators and Overlay 0 `#9ca0b0` for visible borders.

**Accent colors** shift to darker variants for contrast: Blue `#1e66f5`, Sky `#04a5e5` for hover, and Lavender `#7287fd` for muted backgrounds. Mauve `#8839ef` is a vibrant alternative.

**Status colors** are more saturated for light backgrounds: Green `#40a02b`, Yellow `#df8e1d`, and Red `#d20f39`.

---

## Tokyo Night theme

Tokyo Night from folke/tokyonight.nvim creates an aesthetic inspired by Tokyo's city lights, with deep blues and vibrant accents. The "night" variant is the primary dark theme.

### Complete token mapping

**Background layers** use `#1a1b26` (bg) as the base, `#16161e` (bg_dark) for sidebars and panels, `#292e42` (bg_highlight) for elevated inputs, and `#283457` (bg_visual) for selections and hover.

**Text colors** progress through `#c0caf5` (fg) for primary, `#a9b1d6` (fg_dark) for secondary, and `#565f89` (comment) for muted/disabled states.

**Borders** use `#15161e` for subtle separators and `#3b4261` (fg_gutter) for visible borders.

**Accent colors** feature the signature blue `#7aa2f7`, with darker blue `#3d59a1` (blue0) for hover states and `#394b70` (blue7) for muted backgrounds.

**Status colors** include green `#9ece6a`, yellow `#e0af68`, and error red `#db4b4b`.

### Additional Tokyo Night colors
| Color | Hex | Use |
|-------|-----|-----|
| Cyan | `#7dcfff` | Types, operators |
| Teal | `#73daca` | Strings |
| Purple | `#bb9af7` | Keywords |
| Magenta | `#ff007c` | Special |
| Orange | `#ff9e64` | Numbers |

---

## One Dark theme

One Dark originated in Atom editor (atom/one-dark-syntax) and has become one of the most ported themes. Colors were precisely measured by joshdick for onedark.vim.

### Complete token mapping

**Background layers** establish hierarchy with `#282c34` (syntax-bg) as base, `#21252b` for slightly darker panels, `#2c323c` (cursor_grey) for cursor lines and inputs, and `#3e4452` (visual_grey) for selections.

**Text colors** use a three-tier mono system: `#abb2bf` (mono-1) for primary, `#828997` (mono-2) for secondary, and `#5c6370` (mono-3/comment_grey) for muted content.

**Borders** use `#3b4048` (special_grey) for subtle separators and `#4b5263` (gutter_fg) for visible borders.

**Accent colors** center on blue `#61afef` (hue-2), with brighter `#528bff` (syntax-accent) for hover and a derived `#3a4a5e` for muted backgrounds.

**Status colors** are warm and readable: green `#98c379` (hue-4), yellow `#e5c07b` (hue-6), and red `#e06c75` (hue-5).

### Complete One Dark syntax palette
| Color | Hex | Hue Name |
|-------|-----|----------|
| Red (light) | `#e06c75` | hue-5 |
| Red (dark) | `#be5046` | hue-5-2 |
| Green | `#98c379` | hue-4 |
| Yellow (light) | `#e5c07b` | hue-6 |
| Yellow (dark) | `#d19a66` | hue-6-2 |
| Blue | `#61afef` | hue-2 |
| Cyan | `#56b6c2` | hue-1 |
| Purple | `#c678dd` | hue-3 |

---

## Gruvbox Dark theme

Gruvbox from morhetz/gruvbox features a "retro groove" aesthetic with warm, earthy tones. The palette provides hard, medium (default), and soft contrast variants.

### Complete token mapping

**Background layers** progress through dark tones: `#282828` (dark0/bg0) as base, `#3c3836` (dark1/bg1) for surfaces, `#504945` (dark2/bg2) for elevated elements, and `#665c54` (dark3/bg3) for hover. Hard contrast uses `#1d2021`, soft uses `#32302f`.

**Text colors** pull from the light palette: `#ebdbb2` (light1/fg1) for primary, `#d5c4a1` (light2/fg2) for secondary, and `#bdae93` (light3/fg3) for muted. The shared gray `#928374` works for comments.

**Borders** use `#504945` (dark2) for subtle separators and `#7c6f64` (dark4) for visible borders.

**Accent colors** use bright variants for dark mode: blue `#83a598` (bright_blue) or aqua `#8ec07c` (bright_aqua) as primary accents, with neutral variants `#458588` or `#689d6a` for muted backgrounds.

**Status colors** are highly saturated bright variants: green `#b8bb26`, yellow `#fabd2f` (or orange `#fe8019`), and red `#fb4934`.

---

## Gruvbox Light theme

Light Gruvbox inverts the background/foreground relationship while using faded accent variants for proper contrast.

### Complete token mapping

**Background layers** use light tones: `#fbf1c7` (light0) as base, `#ebdbb2` (light1) for surfaces, `#d5c4a1` (light2) for elevated elements, and `#bdae93` (light3) for hover.

**Text colors** use dark values: `#3c3836` (dark1) for primary, `#504945` (dark2) for secondary, and `#665c54` (dark3) for muted.

**Borders** use `#d5c4a1` (light2) for subtle separators and `#a89984` (light4) for visible borders.

**Accent colors** shift to faded variants: blue `#076678` (faded_blue) or aqua `#427b58` (faded_aqua), with neutral variants for hover and bright variants for muted backgrounds.

**Status colors** use darker faded variants: green `#79740e`, yellow `#b57614`, and red `#9d0006`.

---

## Solarized Dark theme

Solarized from Ethan Schoonover features a scientifically designed 16-color palette with precise L*a*b lightness relationships that maintain identical contrast when switching modes.

### Complete token mapping

**Background layers** use base03 `#002b36` as the primary background and base02 `#073642` for elevated surfaces. Solarized's limited palette means bg_hover typically reuses base02.

**Text colors** follow a specific pattern: base0 `#839496` (NOT base00) for primary body text, base1 `#93a1a1` for emphasized content, and base01 `#586e75` for muted/comment text.

**Borders** use base02 `#073642` for subtle separators and base01 `#586e75` for visible borders.

**Accent colors** are shared across modes: blue `#268bd2` as primary, violet `#6c71c4` for hover variation, and cyan `#2aa198` for muted backgrounds.

**Status colors** remain constant: green `#859900`, yellow `#b58900`, and red `#dc322f`.

---

## Solarized Light theme

Light Solarized swaps the base tones while preserving the same accent colors.

### Complete token mapping

**Background layers** invert to base3 `#fdf6e3` as primary and base2 `#eee8d5` for surfaces.

**Text colors** swap accordingly: base00 `#657b83` for primary, base01 `#586e75` for secondary, and base1 `#93a1a1` for muted.

**Accent and status colors** remain identical to dark mode, as Solarized's design ensures proper contrast in both contexts.

---

## Implementation recommendations

When implementing these palettes, consider these patterns from the official documentation:

- **Catppuccin** provides the most comprehensive semantic system with explicit style guide recommendations for each use case
- **Solarized** requires careful attention to the base color relationships—dark mode uses base0 (not base00) for primary text
- **Gruvbox** varies accent saturation by mode—bright variants for dark backgrounds, faded variants for light
- **Dracula** offers functional color variants (less saturated) specifically for UI elements versus syntax highlighting
- **Nord** organizes colors into conceptual groups (Polar Night, Snow Storm, Frost, Aurora) that map naturally to UI concerns

All hex values in this reference are taken from official theme repositories and documentation, ensuring accuracy for production implementation.