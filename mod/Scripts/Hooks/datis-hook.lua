package.cpath = package.cpath..";"..lfs.writedir().."Mods\\tech\\DATIS\\bin\\?.dll;"

local datis = nil
local isStarted = false

function datis_start()
  if not DCS.isServer() then
    log.write("[DATIS]", log.WARNING, "Starting DATIS skipped for not being the Server ...")
    return
  end

  log.write("[DATIS]", log.DEBUG, "Starting ...")

  if _G.Terrain == nil then
    _G.Terrain = require "terrain"
  end

  if datis == nil then
    datis = require "datis"
  end
  datis.start()

  log.write("[DATIS]", log.INFO, "Started")

  if DCS.getPause() then
    datis.pause()
  end
end

function datis_stop()
  if datis ~= nil then
    datis.stop()
    datis = nil
  end
end

function datis_pause()
  if datis ~= nil then
    datis.pause()
  end
end

function datis_unpause()
  if datis ~= nil then
    datis.unpause()
  end
end

function datis_load()
  log.write("[DATIS]", log.DEBUG, "Loading ...")

  local handler = {}

  function handler.onSimulationStop()
    log.write("[DATIS]", log.DEBUG, "Stopping")

    local status, err = pcall(datis_stop)
    if not status then
      log.write("[DATIS]", log.ERROR, "Stop Error: " .. tostring(err))
    end

    isStarted = false
  end

  function handler.onSimulationPause()
    log.write("[DATIS]", log.DEBUG, "Pausing")

    local status, err = pcall(datis_pause)
    if not status then
      log.write("[DATIS]", log.ERROR, "Pause Error: " .. tostring(err))
    end
  end

  function handler.onSimulationResume()
    log.write("[DATIS]", log.DEBUG, "Resuming")

    if datis == nil and not isStarted then
      isStarted = true
      local status, err = pcall(datis_start)
      if not status then
        log.write("[DATIS]", log.ERROR, "Start Error: " .. tostring(err))
      end
    else
      local status, err = pcall(datis_unpause)
      if not status then
        log.write("[DATIS]", log.ERROR, "Unpause Error: " .. tostring(err))
      end
    end
  end

  DCS.setUserCallbacks(handler)

  log.write("[DATIS]", log.INFO, "Loaded")
end

local status, err = pcall(datis_load)
if not status then
  log.write("[DATIS]", log.ERROR, "Load Error: " .. tostring(err))
end