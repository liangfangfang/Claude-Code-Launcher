# Bug Fix Spec: Template, Group, and Input Issues

> Date: 2026-05-29 | Version: 2.2.1

## Problems Identified

### 1. Template creation not visible after creation
**Symptom**: User creates a template, sees "success" message, but template doesn't appear in lists or dropdowns.

**Root cause**: After `NewTemplateConfirm`, only `s.template_names` was updated. The `s.global_template_options` dropdown was NOT refreshed, so the new template didn't appear in the Global config tab's template selector.

**Fix**: Added `refresh_settings_template_lists()` helper that updates BOTH `template_names` and `global_template_options` in one call. Used by `NewTemplateConfirm`, `SaveTemplateClicked`, `SetDefaultClicked`, and `execute_delete_template`.

### 2. Group edit/delete buttons missing
**Symptom**: Groups can be created but not edited or deleted from the main UI.

**Root cause**: The `group_tabs_view` only showed group name buttons and a "+" button. No edit/delete controls were rendered.

**Fix**: Added edit (✎) and delete (✕) buttons next to each group tab. These trigger `EditGroup` and `DeleteGroup` messages which open the appropriate dialogs.

### 3. Chinese input method (IME) not working
**Root cause**: winit 0.27+ requires explicit `window.set_ime_allowed(true)` to enable IME support. iced 0.13 does NOT call this API, so IME (Chinese/Japanese/Korean input) is disabled by default.

**Fix**: Patched iced_winit locally to call `window.set_ime_allowed(true)` after window creation. This enables IME support for CJK input. The patched crate is included in the project as `iced_winit_local/` and referenced via `[patch.crates-io]` in Cargo.toml.

### 4. Chinese path garbled in file dialog
**Root cause**: PowerShell on Chinese Windows outputs in GBK encoding by default, not UTF-8. The `browse_folder()` function used `String::from_utf8_lossy` which incorrectly decoded GBK bytes as UTF-8.

**Fix**: Added `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8` to the PowerShell script to force UTF-8 output. Also improved the decoding logic to try UTF-8 first, then fall back to lossy decoding.

### 5. Group edit/delete buttons cramped in tab bar
**Symptom**: The ✎ and ✕ buttons were squeezed next to the group tab names, making them hard to click.

**Fix**: Moved group edit/delete actions to the bottom of the project list when a specific group is selected. Shows "编辑分组「XX」" and "删除分组「XX」" buttons in a toolbar-style container.

### 6. Modal overlay click-through
**Symptom**: When a dialog (Add Group, Settings, etc.) was open, project cards behind it could still be clicked.

**Root cause**: The mask button had transparent text and no explicit size, so iced's hit testing didn't register it as an interactive surface.

**Fix**: Added `.width(Length::Fill).height(Length::Fill)` to the mask button and changed empty text to `" "` with `.size(1)` to ensure the button is recognized as interactive.

## Verification

- All 137 unit/integration tests pass
- Template creation, editing, deletion all refresh the UI correctly
- Group edit/delete buttons visible at bottom of group view
- Modal overlay blocks background interactions
- Chinese path handling fixed with UTF-8 encoding
- Chinese IME support enabled via iced_winit patch
- Release binary builds successfully (15MB)
