-- Opt-in visible telemetry. Addons cannot write arbitrary host files or open
-- localhost sockets: this panel carries bounded configuration data and textures.
-- Never sends chat, reads macro bodies, or performs protected gameplay actions.
local live, api = {}, assert(ThumbleLink)
local c = api.codec
local MAX_BYTES, FRAME_BYTES, HEADER_BYTES = 32768, 512, 25
local PAYLOAD_BYTES = FRAME_BYTES - HEADER_BYTES
local pads = {
    { "PAD1", "south" }, { "PAD2", "east" }, { "PAD3", "west" }, { "PAD4", "north" },
    { "PADDUP", "dpadUp" }, { "PADDDOWN", "dpadDown" },
    { "PADDLEFT", "dpadLeft" }, { "PADDRIGHT", "dpadRight" },
    { "PADLSHOULDER", "leftShoulder" }, { "PADRSHOULDER", "rightShoulder" },
    { "PADLTRIGGER", "leftTriggerButton" }, { "PADRTRIGGER", "rightTriggerButton" },
    { "PADLSTICK", "leftStickPress" }, { "PADRSTICK", "rightStickPress" },
    { "PADBACK", "select" }, { "PADFORWARD", "start" },
}

function live.checksum(bytes)
    local a, b = 1, 0
    for i = 1, #bytes do a = (a + bytes:byte(i)) % 65521; b = (b + a) % 65521 end
    return b * 65536 + a
end

local function unsigned(value, count)
    local bytes = {}
    for i = 1, count do bytes[i] = string.char(value % 256); value = math.floor(value / 256) end
    return table.concat(bytes)
end

-- Fixed little-endian wire frame; checksums cover each page and the whole JSON.
function live.frames(message, tick)
    assert(type(message) == "string" and #message > 0 and #message <= MAX_BYTES, "live snapshot exceeds bound")
    local count, result, digest = math.ceil(#message / PAYLOAD_BYTES), {}, live.checksum(message)
    for page = 0, count - 1 do
        local payload = message:sub(page * PAYLOAD_BYTES + 1, (page + 1) * PAYLOAD_BYTES)
        local header = "THWL" .. string.char(1) .. unsigned(tick, 4) .. unsigned(digest, 4)
            .. unsigned(page, 2) .. unsigned(count, 2) .. unsigned(#payload, 2)
            .. unsigned(#message, 2)
        header = header .. unsigned(live.checksum(header .. payload), 4)
        result[#result + 1] = header .. payload .. string.rep("\0", FRAME_BYTES - #header - #payload)
    end
    return result
end

local function modifiers(left, right)
    local selected = {}
    for _, entry in ipairs({ { "ALT", "GamePadEmulateAlt" }, { "CTRL", "GamePadEmulateCtrl" },
                              { "SHIFT", "GamePadEmulateShift" } }) do
        local pad = c.text(GetCVar(entry[2]), "", 32)
        if (left and pad == "PADLTRIGGER") or (right and pad == "PADRTRIGGER") then
            selected[#selected + 1] = entry[1] .. "-"
        end
    end
    return table.concat(selected)
end

function live.snapshot()
    if InCombatLockdown() then error("Live action refresh deferred until combat ends") end
    local _, _, _, interface = GetBuildInfo()
    local flavor = ({ [11509] = "classic-era", [16001] = "forever" })[interface]
    assert(flavor, "Unsupported live client interface")
    local guid = c.text(UnitGUID("player"), nil, 96)
    local name, realm = c.text(UnitName("player"), nil, 96), c.text(GetRealmName(), nil, 96)
    local _, class = UnitClass("player"); class = c.text(class, nil, 32)
    assert(guid and name and realm and class, "Waiting for character login")
    local slots = {}
    for _, bar in ipairs(api.bars) do
        for i = 1, 12 do
            local button = _G[bar[1] .. i]
            local slot = button and c.number(button.action)
            if slot then slots[bar[2] .. i] = slot end
        end
    end
    if ConsolePort and ConsolePort.GetActionBinding then
        for slot = 1, 180 do
            local command = c.text(ConsolePort:GetActionBinding(slot), nil, 128)
            -- Actual Blizzard frame resolution wins for page/form-aware commands.
            if command and not slots[command] then slots[command] = slot end
        end
    end
    local bindings = ConsolePort and ConsolePort.GetCurrentBindings and ConsolePort:GetCurrentBindings() or {}
    if type(bindings) ~= "table" then bindings = {} end
    local icons, iconIndex, actions = c.array(), {}, c.array()
    local banks = { { "base", false, false }, { "left-trigger", true, false },
                    { "right-trigger", false, true }, { "both-triggers", true, true } }
    for _, bank in ipairs(banks) do
        local modifier = modifiers(bank[2], bank[3])
        for _, pad in ipairs(pads) do
            local mapping = bindings[pad[1]]
            local command = type(mapping) == "table" and c.text(mapping[modifier], nil, 128)
                or c.text(GetBindingAction(modifier .. pad[1], true), nil, 128)
            if command and command ~= "" then
                local slot = slots[command]
                local kind, id
                if slot then kind, id = GetActionInfo(slot) end
                kind, id = c.text(kind, "utility", 32), c.number(id)
                local textureAPI = C_ActionBar and C_ActionBar.GetActionTexture or GetActionTexture
                local texture = slot and textureAPI and c.number(textureAPI(slot))
                local index
                if texture then
                    index = iconIndex[texture]
                    if not index and #icons < 64 then
                        icons[#icons + 1] = texture; index = #icons; iconIndex[texture] = index
                    end
                end
                actions[#actions + 1] = { bank = bank[1], button = pad[2], command = command,
                    slot = slot, kind = kind, actionID = id, iconFileID = texture, iconIndex = index,
                    label = api.actionLabel(kind, id, slot, command) }
            end
        end
    end
    local devices = c.array()
    if C_GamePad and C_GamePad.GetAllDeviceIDs and C_GamePad.GetDeviceRawState then
        for _, id in ipairs(C_GamePad.GetAllDeviceIDs()) do
            local state = C_GamePad.GetDeviceRawState(id)
            if state and #devices < 16 then
                devices[#devices + 1] = { name = c.text(state.name, "Gamepad", 128),
                    vendorID = c.number(state.vendorID), productID = c.number(state.productID) }
            end
        end
    end
    local snapshot = { schema = "thumble.wow-live", version = 1,
        client = { flavor = flavor, interface = interface },
        character = { id = guid, name = name, realm = realm, classTag = class },
        actions = actions, icons = icons, devices = devices,
        consolePortVersion = c.text(ConsolePort and (C_AddOns and C_AddOns.GetAddOnMetadata or GetAddOnMetadata)("ConsolePort", "Version"), "absent", 32) }
    local encoded = c.json(snapshot)
    assert(#encoded <= MAX_BYTES, "Live action snapshot exceeds bound")
    return encoded, icons
end

-- Pixel protocol: 64x16 two-nibble bytes, 3 UI pixels per cell, calibrated palette.
-- Magenta/cyan fiducials span the full width, avoiding fractional-scale drift.
-- Sixteen gray calibration cells precede the data.
local panel, cells, iconTextures
local function render(bytes)
    for i = 1, FRAME_BYTES do
        local byte = bytes:byte(i)
        local hi, lo = math.floor(byte / 16) / 15, (byte % 16) / 15
        cells[i * 2 - 1]:SetColorTexture(hi, hi, hi, 1)
        cells[i * 2]:SetColorTexture(lo, lo, lo, 1)
    end
end

local function createPanel()
    local frame = CreateFrame("Frame", "ThumbleLinkLivePanel", UIParent)
    frame:SetSize(256, 320); frame:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 8, -8)
    frame:SetFrameStrata("TOOLTIP"); frame:EnableMouse(false)
    local background = frame:CreateTexture(nil, "BACKGROUND")
    background:SetAllPoints(); background:SetColorTexture(0, 0, 0, 1)
    local function cell(x, y, width, height)
        local t = frame:CreateTexture(nil, "OVERLAY")
        t:SetPoint("TOPLEFT", x, -y); t:SetSize(width, height); return t
    end
    for i = 0, 3 do
        cell(i * 6, 0, 6, 6):SetColorTexture(i % 2, 1 - i % 2, 1, 1)
    end
    for i = 0, 3 do
        cell(240 + i * 4, 0, 4, 6):SetColorTexture(i % 2, 1 - i % 2, 1, 1)
    end
    for i = 0, 15 do local n = i / 15; cell(24 + i * 6, 0, 6, 6):SetColorTexture(n, n, n, 1) end
    cells, iconTextures = {}, {}
    for i = 0, FRAME_BYTES * 2 - 1 do cells[i + 1] = cell((i % 64) * 3, 8 + math.floor(i / 64) * 3, 3, 3) end
    for i = 0, 63 do
        local t = cell((i % 8) * 32, 64 + math.floor(i / 8) * 32, 32, 32)
        t:SetTexture(nil); iconTextures[i + 1] = t
    end
    frame:Hide(); return frame
end

local pages, page, tick = nil, 1, 0
local function update()
    if not panel or not panel:IsShown() then return end
    if (not pages or page == 1) and not InCombatLockdown() then
        local ok, bytes, icons = pcall(live.snapshot)
        if ok then
            pages = live.frames(bytes, tick)
            for i = 1, 64 do iconTextures[i]:SetTexture(icons[i]) end
        else
            -- Never keep advertising another character after login failure.
            pages = nil; panel:Hide(); live.lastError = tostring(bytes); return
        end
    end
    if pages then
        -- Tick is renewed every displayed page, even for unchanged snapshots.
        tick = (tick + 1) % 4294967296
        local bytes = pages[page]
        local header = bytes:sub(1, 5) .. unsigned(tick, 4) .. bytes:sub(10, 21)
        local length = bytes:byte(18) + bytes:byte(19) * 256
        bytes = header .. unsigned(live.checksum(header .. bytes:sub(26, 25 + length)), 4) .. bytes:sub(26)
        render(bytes); page = page % #pages + 1
    end
end

function live.enable(enabled)
    if not panel then panel = createPanel() end
    ThumbleLinkSettings = ThumbleLinkSettings or {}
    ThumbleLinkSettings.liveEnabled = enabled and true or false
    pages, page = nil, 1
    if enabled then panel:Show(); update() else panel:Hide() end
end

SLASH_THUMBLELIVE1 = "/thumblelive"
SlashCmdList.THUMBLELIVE = function(command)
    if command == "off" then live.enable(false)
    elseif command == "" or command == "on" then live.enable(true)
    elseif print then print("Thumble Link: /thumblelive on or /thumblelive off. Use /thumble for a keyboard export.") end
end

if CreateFrame then
    local events = CreateFrame("Frame")
    for _, event in ipairs({ "PLAYER_ENTERING_WORLD", "PLAYER_LEAVING_WORLD", "PLAYER_LOGOUT", "ACTIONBAR_SLOT_CHANGED",
        "UPDATE_BINDINGS", "ACTIONBAR_PAGE_CHANGED", "UPDATE_SHAPESHIFT_FORM", "PLAYER_REGEN_ENABLED" }) do events:RegisterEvent(event) end
    events:SetScript("OnEvent", function(_, event)
        if event == "PLAYER_LEAVING_WORLD" or event == "PLAYER_LOGOUT" then
            pages = nil; if panel then panel:Hide() end
        elseif event == "PLAYER_ENTERING_WORLD" then
            if ThumbleLinkSettings and ThumbleLinkSettings.liveEnabled then live.enable(true) end
        end
    end)
    local elapsed = 0
    events:SetScript("OnUpdate", function(_, delta)
        elapsed = elapsed + delta
        if elapsed >= 0.2 then elapsed = 0; update() end
    end)
end

api.live = live
return live
