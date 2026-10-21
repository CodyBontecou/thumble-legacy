-- ConsolePort 3.3.11 adapter. No arbitrary host code, combat automation, or
-- tutorial-completion flags. Existing keyboard bindings are checked after apply.
local api = _G.ThumbleLink
if not api then return end
local onboarding = {}
api.onboarding = onboarding
local pads = { "PAD1", "PAD2", "PAD3", "PAD4", "PADDUP", "PADDDOWN", "PADDLEFT", "PADDRIGHT",
    "PADLSHOULDER", "PADRSHOULDER", "PADLTRIGGER", "PADRTRIGGER", "PADLSTICK", "PADRSTICK", "PADBACK", "PADFORWARD" }
local modifiers = { "", "ALT-", "CTRL-", "SHIFT-", "ALT-CTRL-", "ALT-SHIFT-", "CTRL-SHIFT-", "ALT-CTRL-SHIFT-" }
local desired = { GamePadEnable = "1", GamePadEmulateShift = "PADLTRIGGER", GamePadEmulateCtrl = "PADRTRIGGER",
    GamePadEmulateAlt = "NONE", GamePadCursorLeftClick = "PADLSTICK", GamePadCursorRightClick = "PADRSTICK" }
local function version()
    if C_AddOns and C_AddOns.GetAddOnMetadata then return C_AddOns.GetAddOnMetadata("ConsolePort", "Version") end
    return GetAddOnMetadata and GetAddOnMetadata("ConsolePort", "Version")
end
local function state()
    ThumbleLinkSettings = ThumbleLinkSettings or {}
    ThumbleLinkSettings.onboarding = ThumbleLinkSettings.onboarding or {}
    local id = UnitGUID and UnitGUID("player")
    if not id or not id:match("^Player%-") then return nil end
    local records = ThumbleLinkSettings.onboarding
    if not records[id] then
        local count = 0; for _ in pairs(records) do count = count + 1 end
        if count >= 256 then return nil end
        records[id] = { status = "pending" }
    end
    return records[id]
end
local function status(record, value)
    if record then record.status = value end
    return value
end
local function devicePresent()
    if not C_GamePad or not C_GamePad.GetAllDeviceIDs or not C_GamePad.GetDeviceRawState then return false end
    for _, id in ipairs(C_GamePad.GetAllDeviceIDs()) do
        local raw = C_GamePad.GetDeviceRawState(id)
        if raw and raw.vendorID == 1118 and raw.productID == 654 then return true end
    end
    return false
end
local function padBindings()
    local result, any = {}, false
    for _, pad in ipairs(pads) do for _, modifier in ipairs(modifiers) do
        local key = modifier .. pad
        local binding = GetBindingAction(key, false) or ""
        result[key] = binding
        if binding ~= "" then any = true end
    end end
    return result, any
end
local function keyboardBindings()
    local result = {}
    local count = GetNumBindings()
    assert(count <= 4096, "Too many binding declarations")
    for i = 1, count do
        local _, _, first, second = GetBinding(i)
        for _, key in pairs({ first, second }) do
            if not key:match("^PAD") and not key:find("-PAD", 1, true) then result[key] = GetBindingAction(key, false) end
        end
    end
    return result
end
local function restore(record)
    for key, binding in pairs(record.before.pad) do assert(SetBinding(key, binding ~= "" and binding or nil), "Controller restore failed") end
    for key, binding in pairs(record.before.keyboard) do assert(SetBinding(key, binding ~= "" and binding or nil), "Keyboard restore failed") end
    for key, value in pairs(record.before.cvars) do SetCVar(key, value) end
    SaveBindings(2)
    if LoadBindings then LoadBindings(record.before.bindingSet) end
end
local function step(force)
    local setup = ThumbleLinkSetup
    if not setup or setup.version ~= 1 or not setup.enabled then return "disabled" end
    local record = state()
    if not record then return "waiting-for-character" end
    if record.suspended and not force then
        if record.status == "restore-deferred-combat" and not InCombatLockdown() then return onboarding.restore() end
        return record.status
    end
    if record.status == "applying" and not force then return "applying" end
    if setup.enableLive and ThumbleLinkSettings.liveEnabled == nil and api.live then api.live.enable(true) end
    if record.status == "configured" and not force then return record.status end
    if InCombatLockdown() then return status(record, "deferred-combat") end
    if version() ~= "3.3.11" then return status(record, "unsupported-consoleport") end
    if not ConsolePort or not ConsolePort.GetData then return status(record, "waiting-for-consoleport") end
    if record.before and record.status ~= "restored" and not force then return status(record, "recovery-required") end
    local set = GetCurrentBindingSet()
    if set ~= 1 and set ~= 2 then return status(record, "waiting-for-bindings") end
    -- Enumeration can require the gamepad subsystem to be enabled first.
    -- Preserve this bootstrap separately even if no device is available yet.
    if GetCVar("GamePadEnable") ~= "1" then
        if record.bootstrapBefore == nil then record.bootstrapBefore = GetCVar("GamePadEnable") end
        SetCVar("GamePadEnable", "1")
    end
    if not devicePresent() then return status(record, "waiting-for-thumble-gamepad") end
    local before, any = padBindings()
    if any and not force and not setup.replaceBindings then return status(record, "keeping-existing-controller-bindings") end
    local db = ConsolePort:GetData()
    local gamepad = db and db.Gamepad
    if not gamepad or not gamepad.GetDevice or not gamepad.SetActiveDevice or not gamepad.ReindexMappedState then
        return status(record, "unsupported-consoleport-api")
    end
    local device = gamepad:GetDevice("Xbox")
    if not device or not device.GetPresetBindings then return status(record, "unsupported-xbox-preset") end
    local oldCVars = {}; for key in pairs(desired) do oldCVars[key] = GetCVar(key) end
    oldCVars.GamePadEnable = record.bootstrapBefore or oldCVars.GamePadEnable
    local attemptBefore = { pad = before, keyboard = keyboardBindings(), cvars = oldCVars, bindingSet = set }
    if not record.before or record.status == "restored" then record.before = attemptBefore end
    record.suspended = nil
    record.status = "applying"
    local ok, errorMessage = pcall(function()
        for key, value in pairs(desired) do
            SetCVar(key, value)
            assert(tostring(GetCVar(key)):upper() == value:upper(), "WoW rejected a controller setting")
        end
        gamepad:ReindexMappedState(true)
        local preset = device:GetPresetBindings()
        local known = {}; for key in pairs(before) do known[key] = true end
        local count = 0
        for key, binding in pairs(preset) do
            if known[key] then
                assert(type(binding) == "string" and #binding <= 128, "Invalid preset binding")
                assert(SetBinding(key, binding ~= "" and binding or nil), "WoW rejected a controller binding")
                count = count + 1
            end
        end
        assert(count > 0 and count <= 128, "Invalid preset size")
        for key, binding in pairs(attemptBefore.keyboard) do
            assert(GetBindingAction(key, false) == binding, "Preset would change a keyboard binding")
        end
        gamepad:SetActiveDevice("Xbox")
        SaveBindings(2) -- Character-local copy; the account binding file is not replaced.
        if LoadBindings then LoadBindings(2) end
        for key, binding in pairs(attemptBefore.keyboard) do
            assert(GetBindingAction(key, false) == binding, "Saved preset would change a keyboard binding")
        end
    end)
    record.bootstrapBefore = nil
    if not ok then
        local restored = pcall(restore, { before = attemptBefore })
        record.error = tostring(errorMessage):sub(1, 256)
        return status(record, restored and "apply-failed-restored" or "recovery-required")
    end
    record.error = nil
    return status(record, "configured")
end
local busy = false
function onboarding.step(force)
    if busy then return "applying" end
    busy = true
    local ok, result = pcall(step, force)
    busy = false
    if not ok then error(result, 0) end
    return result
end

function onboarding.restore()
    local record = state()
    if not record or (not record.before and record.bootstrapBefore == nil) then return "no-backup" end
    record.suspended = true -- Binding/CVar events must not immediately reapply it.
    if InCombatLockdown() then return status(record, "restore-deferred-combat") end
    local ok = pcall(function()
        if record.before then restore(record)
        else SetCVar("GamePadEnable", record.bootstrapBefore) end
    end)
    if ok then record.bootstrapBefore = nil end
    return status(record, ok and "restored" or "recovery-required")
end

SLASH_THUMBLESETUP1 = "/thumblesetup"
SlashCmdList.THUMBLESETUP = function(command)
    local result
    if command == "apply" then result = onboarding.step(true)
    elseif command == "restore" then result = onboarding.restore()
    else local record = state(); result = record and record.status or "waiting-for-character" end
    if print then print("Thumble setup: " .. result) end
end
if CreateFrame then
    local events = CreateFrame("Frame")
    for _, event in ipairs({ "PLAYER_ENTERING_WORLD", "PLAYER_REGEN_ENABLED", "UPDATE_BINDINGS", "GAME_PAD_CONFIGS_CHANGED", "CVAR_UPDATE" }) do
        events:RegisterEvent(event)
    end
    local applying = false
    events:SetScript("OnEvent", function(_, event, key)
        if event == "CVAR_UPDATE" and (type(key) ~= "string" or key:lower() ~= "gamepadenable") then return end
        if applying then return end
        applying = true
        local ok = pcall(onboarding.step)
        applying = false
        if not ok then local record = state(); if record then record.suspended = true end; status(record, "recovery-required") end
    end)
end
return onboarding
