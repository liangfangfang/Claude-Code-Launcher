# Implementation Plan: Bug Fix Round

> Date: 2026-05-29 | Version: 2.2.1

## 1. Problem Summary

| # | Issue | Priority |
|---|-------|----------|
| 1 | Template creation: new template not visible in lists/dropdowns | P0 |
| 2 | Group edit/delete: no UI controls for editing/deleting groups | P0 |
| 3 | Group assignment: existing projects cannot change group | P0 |
| 4 | Edit project: no group selection in edit dialog | P0 |
| 5 | Chinese IME: cannot input Chinese characters via IME | P1 |
| 6 | Chinese path: garbled Chinese paths in file dialog | P1 |

## 2. Root Cause Analysis

### Issue 1: Template not visible after creation
- **Root cause**: `NewTemplateConfirm` handler updated `s.template_names` but NOT `s.global_template_options`
- **Evidence**: Code at `app.rs:842-845` only updates `template_names`
- **Fix**: Add `refresh_settings_template_lists()` helper

### Issue 2: No group edit/delete UI
- **Root cause**: `group_tabs_view` only rendered group name buttons, no edit/delete controls
- **Evidence**: Code at `main_view.rs:121-134` only creates group buttons
- **Fix**: Add edit (✎) and delete (✕) buttons next to each group tab

### Issue 3: Cannot change project group
- **Root cause**: `update_project()` only supports `name` and `path` fields
- **Evidence**: Function signature at `project_manager.rs:222` lacks `group_id` parameter
- **Fix**: Add `update_project_group()` function

### Issue 4: No group selection in edit dialog
- **Root cause**: `edit_project::State` lacked group-related fields
- **Evidence**: Code at `edit_project.rs:10-16` has no `group_options` or `selected_group`
- **Fix**: Add group selection dropdown to edit dialog

### Issue 5: Chinese IME not working
- **Root cause**: winit 0.27+ requires `window.set_ime_allowed(true)` but iced 0.13 doesn't call it
- **Evidence**: Checked iced_winit 0.13 source - no `set_ime_allowed` call
- **Fix**: Patch iced_winit locally to call `set_ime_allowed(true)` after window creation

### Issue 6: Chinese path garbled
- **Root cause**: PowerShell outputs in GBK encoding by default on Chinese Windows
- **Evidence**: `browse_folder()` used `String::from_utf8_lossy` on GBK bytes
- **Fix**: Add `[Console]::OutputEncoding = UTF-8` to PowerShell script

## 3. Implementation Steps

### Step 1: Fix template visibility
- Add `refresh_settings_template_lists()` helper function
- Update `NewTemplateConfirm`, `SaveTemplateClicked`, `SetDefaultClicked`, `execute_delete_template`
- Refactor `OpenSettings` to use `open_settings_dialog()`

### Step 2: Add group edit/delete UI
- Modify `group_tabs_view` in `main_view.rs` to add ✎ and ✕ buttons
- Wire buttons to `EditGroup` and `DeleteGroup` messages

### Step 3: Fix project group management
- Add `update_project_group()` to `project_manager.rs`
- Update `handle_add_project` to use `update_project_group`
- Update `handle_edit_project` to use `update_project_group`

### Step 4: Add group selection to edit dialog
- Modify `edit_project::State` to add `group_options`, `selected_group`
- Add `GroupSelected` message variant
- Update `EditProject` handler to pass group data

### Step 5: Fix Chinese IME
- Copy iced_winit 0.13 source locally
- Add `window.set_ime_allowed(true)` after window creation
- Add `[patch.crates-io]` to Cargo.toml

### Step 6: Fix Chinese path encoding
- Add `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8` to PowerShell script
- Improve fallback decoding logic

## 4. Verification

### Automated tests
- All 137 unit/integration tests pass
- Template creation, editing, deletion tested
- Group creation, editing, deletion tested
- Project group assignment tested

### Manual verification (screenshot)
- Application renders correctly with Chinese text
- Admin status badge visible
- Group tabs with edit/delete buttons visible
- Project cards with checkboxes visible
- All buttons functional

### Build verification
- Release binary: 15MB
- All dependencies resolved
- No compilation errors

## 5. Files Changed

| File | Change |
|------|--------|
| `src/gui/app.rs` | Add `refresh_settings_template_lists()`, `open_settings_dialog()`, update handlers, fix template creation/save with close-and-reopen pattern, fix modal overlay mask |
| `src/gui/main_view.rs` | Move group edit/delete buttons from tab bar to bottom of group view |
| `src/gui/dialogs/edit_project.rs` | Add group selection dropdown |
| `src/core/project_manager.rs` | Add `update_project_group()` function |
| `Cargo.toml` | Add `[patch.crates-io]` for iced_winit |
| `iced_winit_local/src/program.rs` | Add `set_ime_allowed(true)` |
| `src/gui/app.rs` | Fix `browse_folder()` UTF-8 encoding |

## 6. Additional Fixes (v2.2.1 iteration 2)

| Issue | Fix |
|-------|-----|
| Template creation not visible after save | Close and reopen Settings dialog after create/save (same as delete pattern) |
| Group buttons cramped in tab bar | Move to bottom of group project view |
| Modal click-through | Make mask button fill screen with opaque background |
| Old config format incompatible | Startup validation: check each config file, archive if incompatible |

## 7. Config Migrator (v2.2.1 iteration 3)

| File | Change |
|------|--------|
| `src/core/config_migrator.rs` | New module: config directory validation, format checking, archive & reinitialize |
| `src/core/mod.rs` | Register config_migrator module |
| `src/main.rs` | Add `check_config_compatibility()` before GUI startup |

### Validation rules
- `projects.json`: must be `{id: {...}}` or empty; `{"projects": [...]}` → incompatible
- `groups.json`: must be `{id: {...}}` or empty
- `templates.json`: must have `default_template_id` + `templates`; `{name, content}` → incompatible

### Archive flow
1. Validate each config file
2. If incompatible found → native MessageBox (Yes/No)
3. User confirms → rename dir to `.claude-launcher_backup_<timestamp>`
4. Create fresh empty config directory
5. Continue normal startup
