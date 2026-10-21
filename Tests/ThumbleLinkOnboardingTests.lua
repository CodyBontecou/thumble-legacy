local root = assert(arg[1], "repository root required")
local function copy(source) local result = {}; for k,v in pairs(source) do result[k] = v end; return result end
local bindings, saved, cvars, guid, combat, recognized, addonVersion, saves, failOnce, removeKeyboard
local activeSet, selected, reindexes, liveEnabled, onCVar
local function reset()
    bindings, saved, cvars = { W = "MOVEFORWARD", SPACE = "JUMP", NUMPAD0 = "ACTIONBUTTON2" }, {}, { GamePadEnable = "0" }
    guid, combat, recognized, addonVersion = "Player-1-1", false, true, "3.3.11"
    saves, reindexes, liveEnabled, selected, activeSet = 0, 0, 0, nil, 1
    failOnce, removeKeyboard, onCVar = false, false, nil
    ThumbleLinkSettings = nil
    ThumbleLinkSetup = { version = 1, enabled = true, enableLive = true, replaceBindings = false }
    _G.ThumbleLink = { live = { enable = function() liveEnabled = liveEnabled + 1; ThumbleLinkSettings.liveEnabled = true end } }
    saved[1] = copy(bindings)
end
function UnitGUID() return guid end
function InCombatLockdown() return combat end
function GetCurrentBindingSet() return activeSet end
function GetBindingAction(key) return bindings[key] or "" end
function GetCVar(key) return cvars[key] or "NONE" end
function SetCVar(key, value) cvars[key] = value; if onCVar then onCVar(key) end end
function GetNumBindings() return 3 end
function GetBinding(index) if index == 1 then return "MOVEFORWARD", "Movement", "W" elseif index == 2 then return "JUMP", "Movement", "SPACE" else return "ACTIONBUTTON2", "Action", "NUMPAD0" end end
function SetBinding(key, value)
    if failOnce and value == "JUMP" and key == "PAD1" then failOnce = false; return false end
    bindings[key] = value
    if removeKeyboard and key == "PAD1" and value == "JUMP" then bindings.NUMPAD0 = nil end
    return true
end
function SaveBindings(set) assert(set == 2, "Never save the account binding file"); saves = saves + 1; saved[set] = copy(bindings); activeSet = set end
function LoadBindings(set) activeSet = set; bindings = copy(saved[set] or {}) end
C_AddOns = { GetAddOnMetadata = function() return addonVersion end }
C_GamePad = { GetAllDeviceIDs = function() return recognized and cvars.GamePadEnable == "1" and { 1 } or {} end,
    GetDeviceRawState = function() return { vendorID = 1118, productID = 654 } end }
local device = { GetPresetBindings = function()
    assert(cvars.GamePadEmulateShift == "PADLTRIGGER" and cvars.GamePadEmulateCtrl == "PADRTRIGGER")
    return { PAD1 = "JUMP", ["SHIFT-PAD1"] = "ACTIONBUTTON1", PAD5 = "IGNORED_UNAVAILABLE_BUTTON" }
end }
local gamepad = { GetDevice = function(_, name) assert(name == "Xbox"); return device end,
    SetActiveDevice = function(_, name) selected = name end,
    ReindexMappedState = function() reindexes = reindexes + 1 end }
ConsolePort = { GetData = function() return { Gamepad = gamepad } end }
SlashCmdList = {}
local function load() return assert(loadfile(root .. "/Addons/ThumbleLink/ThumbleLinkOnboarding.lua"))() end
reset(); local setup = load()
combat = true; assert(setup.step() == "deferred-combat" and saves == 0)
combat = false; recognized = false; assert(setup.step() == "waiting-for-thumble-gamepad" and saves == 0)
recognized = true; addonVersion = "4.0"; assert(setup.step() == "unsupported-consoleport" and saves == 0)
addonVersion = "3.3.11"; assert(setup.step() == "configured")
assert(bindings.PAD1 == "JUMP" and bindings["SHIFT-PAD1"] == "ACTIONBUTTON1" and bindings.PAD5 == nil)
assert(bindings.W == "MOVEFORWARD" and bindings.SPACE == "JUMP" and saved[1].PAD1 == nil)
assert(saves == 1 and activeSet == 2 and selected == "Xbox" and liveEnabled == 1 and reindexes == 1)
assert(setup.step() == "configured" and saves == 1)
combat = true; assert(setup.step() == "configured" and saves == 1); combat = false
assert(setup.restore() == "restored" and bindings.PAD1 == nil and activeSet == 1)
assert(cvars.GamePadEnable == "0" and saved[1].PAD1 == nil)
local priorSaves = saves
assert(setup.step() == "restored" and saves == priorSaves and cvars.GamePadEnable == "0", "Restore must survive subsequent binding/CVar events")
reset(); setup = load(); bindings.PAD1 = "CUSTOM_EXISTING"
assert(setup.step() == "keeping-existing-controller-bindings" and saves == 0 and bindings.PAD1 == "CUSTOM_EXISTING")
assert(setup.step(true) == "configured" and saves == 1)
assert(setup.step(true) == "configured" and saves == 2)
assert(ThumbleLinkSettings.onboarding[guid].before.pad.PAD1 == "CUSTOM_EXISTING", "Repeated apply must keep the original restore snapshot")
assert(setup.restore() == "restored" and saved[2].PAD1 == "CUSTOM_EXISTING")
reset(); setup = load(); failOnce = true
assert(setup.step() == "apply-failed-restored" and bindings.PAD1 == nil and bindings.W == "MOVEFORWARD")
assert(setup.step() == "recovery-required")
reset(); setup = load(); removeKeyboard = true
assert(setup.step() == "apply-failed-restored" and bindings.NUMPAD0 == "ACTIONBUTTON2" and saved[1].PAD1 == nil)
reset(); setup = load(); ThumbleLinkSetup.enabled = false
assert(setup.step() == "disabled" and saves == 0)
reset(); setup = load(); recognized = false
assert(setup.step() == "waiting-for-thumble-gamepad" and cvars.GamePadEnable == "1")
assert(setup.restore() == "restored" and cvars.GamePadEnable == "0" and saves == 0)
assert(setup.step() == "restored" and cvars.GamePadEnable == "0")
reset(); setup = load(); assert(setup.step() == "configured"); combat = true
assert(setup.restore() == "restore-deferred-combat"); combat = false
assert(setup.step() == "restored" and cvars.GamePadEnable == "0")
reset(); setup = load(); local reentries = 0
onCVar = function() reentries = reentries + 1; assert(setup.step() == "applying") end
assert(setup.step(true) == "configured" and saves == 1 and reindexes == 1 and reentries > 0)
onCVar = nil
print("ConsolePort onboarding: version/device/combat guards, character-local apply, keyboard preservation and restore passed")
