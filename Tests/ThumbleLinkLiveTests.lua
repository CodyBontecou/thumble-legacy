-- Lua 5.1; exercises production snapshot/transport without a WoW client.
local root = assert(arg[1])
SlashCmdList, UISpecialFrames = {}, {}
local combat, interface, guid = false, 11509, "Player-1234-00000001"
function InCombatLockdown() return combat end
function GetBuildInfo() return "1.15.9", "70003", "", interface end
function UnitGUID() return guid end
function UnitName() return "Live Hunter" end
function GetRealmName() return "Test Realm" end
function UnitClass() return "Hunter", "HUNTER" end
local secret = {}
function issecretvalue(v) return v == secret end
function canaccessvalue(v) return v ~= secret end
local cvars = { GamePadEmulateShift = "PADLTRIGGER", GamePadEmulateCtrl = "PADRTRIGGER", GamePadEmulateAlt = "none" }
function GetCVar(name) return cvars[name] end
function GetBindingAction() return "" end
function GetActionInfo(slot) return "spell", slot == 1 and 78 or 100 end
function GetActionTexture(slot) return slot == 1 and 132282 or 132337 end
function GetActionText(slot)
    assert(type(slot) == "number" and slot > 0 and slot % 1 == 0,
        "Usage: local text = C_ActionBar.GetActionText(actionID)")
    return nil
end
C_Spell = { GetSpellName = function(id) return id == 78 and "Heroic Strike" or "Charge" end }
ActionButton1, ActionButton2 = { action = 1 }, { action = 2 }
local bindings = { PAD1 = { [""] = "JUMP", ["SHIFT-"] = "ACTIONBUTTON1", ["CTRL-"] = "ACTIONBUTTON2",
    ["CTRL-SHIFT-"] = "ACTIONBUTTON1" }, PAD3 = { [""] = "ACTIONBUTTON1", ["SHIFT-"] = "ACTIONBUTTON2" } }
ConsolePort = {
    GetCurrentBindings = function() return bindings end,
    GetActionBinding = function() return nil end,
}
function GetAddOnMetadata() return "3.3.11" end
C_GamePad = { GetAllDeviceIDs = function() return { 1 } end,
    GetDeviceRawState = function() return { name = "Thumble Virtual Gamepad", vendorID = 1118, productID = 654 } end }
local api = assert(loadfile(root .. "/Addons/ThumbleLink/ThumbleLink.lua"))()
local live = assert(loadfile(root .. "/Addons/ThumbleLink/ThumbleLinkLive.lua"))()
local message, icons = live.snapshot()
assert(#icons == 2 and icons[1] == 132282 and icons[2] == 132337, "deduplicate rendered icon atlas")
assert(message:find('"bank":"left-trigger","button":"south"', 1, true))
assert(message:find('"bank":"both-triggers","button":"south"', 1, true))
assert(message:find('"consolePortVersion":"3.3.11"', 1, true))
assert(not message:find("macro body", 1, true))
assert(live.checksum("Wikipedia") == 300286872, "Adler32 reference vector")
local frames = live.frames(message, 123)
assert(#frames == math.ceil(#message / 487))
local pieces = {}
for i, frame in ipairs(frames) do
    assert(#frame == 512 and frame:sub(1, 5) == "THWL\1")
    local length = frame:byte(18) + frame:byte(19) * 256
    assert(length > 0 and length <= 487)
    pieces[i] = frame:sub(26, 25 + length)
end
assert(table.concat(pieces) == message, "lossless paged UTF-8 snapshot")
assert(not pcall(live.frames, string.rep("x", 32769), 0))
combat = true; assert(not pcall(live.snapshot), "do not inspect combat-restricted state"); combat = false
interface = 120100; assert(not pcall(live.snapshot), "unknown client fails closed"); interface = 11509
guid = nil; assert(not pcall(live.snapshot), "no stale character after logout"); guid = "Player-1234-00000001"
bindings.PAD1["SHIFT-"] = secret
assert(not live.snapshot():find('"bank":"left-trigger","button":"south"', 1, true), "secret binding omitted")
bindings.PAD1["SHIFT-"] = "ACTIONBUTTON1"
-- Stable cross-language fixture: JSON line followed by fixed-size binary frames in hex.
io.write(message, "\n")
for _, frame in ipairs(frames) do
    io.write((frame:gsub(".", function(ch) return string.format("%02x", ch:byte()) end)), "\n")
end
