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
  else
    datis.resume()
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

function datis_resume()
  if datis ~= nil then
    datis.resume()
  end
end

local i = 0

function datis_simulation_frame()
  i = i + 1
  if i > 200 then -- roughly every 2 seconds
    i = 0

    local ok, err = pcall(datis_next)
    if not ok then
      log.write("[DATIS]", log.ERROR, "Next error: " .. tostring(err))
    end
  end
end

function datis_next()
  if datis ~= nil then
    datis.try_next(datis_handleRequest)
  end
end

function datis_handleRequest(method, params)
  -- log.write("[DATIS]", log.INFO, "RECV " .. method .. " " .. params)

  if params ~= nil then
    params = net.json2lua(params)
  end

  if method == "get_weather" then
    local position = {
      x = params.x,
      y = params.alt,
      z = params.y,
    }
    local wind = Weather.getGroundWindAtPoint({
      position = position
    })
    local temp, pressure = Weather.getTemperatureAndPressureAtPoint({
      position = position
    })

    return {
      result = net.lua2json({
        windSpeed = wind.v,
        windDir = wind.a,
        temp = temp,
        pressure = pressure,
      })
    }

  elseif method == "get_unit_position" then
    local get_unit_position = [[
      local unit = Unit.getByName("]] .. params.name .. [[")
      if unit == nil then
        return ""
      else
        local pos = unit:getPoint()
        return  pos.x .. ":" .. pos.y .. ":" .. pos.z
      end
    ]]

    local result = net.dostring_in("server", get_unit_position)

    if result == "" then
      return {
        error = "unit not found"
      }
    end

    local x, y, z = string.match(result, "(%-?[0-9%.-]+):(%-?[0-9%.]+):(%-?[0-9%.]+)")

    return {
      result = net.lua2json({
        x = tonumber(x),
        y = tonumber(y),
        z = tonumber(z),
      })
    }

  elseif method == "get_unit_heading" then
    -- north correction is based on https://github.com/mrSkortch/MissionScriptingTools
    local get_unit_heading = [[
      local unit = Unit.getByName("]] .. params.name .. [[")
      if unit == nil then
        return ""
      else
        local unit_pos = unit:getPosition()
        local lat, lon = coord.LOtoLL(unit_pos.p)
        local north_pos = coord.LLtoLO(lat + 1, lon)
        local northCorrection = math.atan2(north_pos.z - unit_pos.p.z, north_pos.x - unit_pos.p.x)

        local heading = math.atan2(unit_pos.x.z, unit_pos.x.x) + northCorrection
        if heading < 0 then
          heading = heading + 2*math.pi
        end

        return tostring(heading)
      end
    ]]
    local result = net.dostring_in("server", get_unit_heading)
    if result == "" then
      return {
        error = "unit not found"
      }
    end

    return {
      result = net.lua2json(tonumber(result))
    }

  elseif method == "get_abs_time" then
    local get_abs_time = [[
      return tostring(timer.getAbsTime())
    ]]

    local result = net.dostring_in("server", get_abs_time)

    return {
      result = net.lua2json(tonumber(result))
    }

  elseif method == "to_lat_lng" then
    local to_lat_lng = [[
      local lat, lng, alt = coord.LOtoLL({ x = ]] .. params.x .. [[, y = ]] .. params.alt .. [[, z = ]] .. params.y .. [[ })
      return lat .. ":" .. lng .. ":" .. alt
    ]]
    local result = net.dostring_in("server", to_lat_lng)
    local lat, lng, alt = string.match(result, "(%-?[0-9%.-]+):(%-?[0-9%.]+):(%-?[0-9%.]+)")
    return {
      result = net.lua2json({
        lat = tonumber(lat),
        lng = tonumber(lng),
        alt = tonumber(alt),
      })
    }

  else
    return {
      error = "unknown method "..method
    }
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
      local status, err = pcall(datis_resume)
      if not status then
        log.write("[DATIS]", log.ERROR, "Unpause Error: " .. tostring(err))
      end
    end
  end

  function handler.onSimulationFrame()
    local status, err = pcall(datis_simulation_frame)
    if not status then
      log.write("[DATIS]", log.ERROR, "Simulation frame Error: " .. tostring(err))
    end
  end

  DCS.setUserCallbacks(handler)

  log.write("[DATIS]", log.INFO, "Loaded")
end

local status, err = pcall(datis_load)
if not status then
  log.write("[DATIS]", log.ERROR, "Load Error: " .. tostring(err))
end