DATIS = {}

local datisPath = lfs.writedir()..[[Scripts\DATIS\]]
package.cpath = package.cpath..';'..datisPath..'?.dll;'

local datis = nil

function datis_start()
	if not DCS.isServer() then
		DATIS.log("Loading Datis skipped for not being the Server ...")
		return
	end

	DATIS.log("Starting ...")

	if datis == nil then
		datis = require 'datis'
	end
	datis.start()

	DATIS.log("Started")

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
	log.write('[DATIS]', log.INFO, 'Loading ...')

	DATIS.logFile = io.open(lfs.writedir()..[[Logs\DATIS.log]], "w")
	function DATIS.log(str)
	    if DATIS.logFile then
	        DATIS.logFile:write(str.."\n")
	        DATIS.logFile:flush()
	    end
	end

	local handler = {}

    function handler.onSimulationStart()
	    DATIS.log("Simulation Start")

		if not DCS.isServer() then
			DATIS.log("Starting DATIS skipped for not being the Server ...")
			return
		end

		local status, err = pcall(datis_start)
		if not status then
			DATIS.log("Start Error: " .. tostring(err))
		end
    end

    -- function handler.onMissionLoadEnd()
	   --  DATIS.log("Mission Load End")
    -- end

    function handler.onSimulationStop()
	    DATIS.log("Simulation Stop")

		local status, err = pcall(datis_stop)
		if not status then
			DATIS.log("Stop Error: " .. tostring(err))
		end
    end

    function handler.onSimulationPause()
	    DATIS.log("Simulation Pause")

		local status, err = pcall(datis_pause)
		if not status then
			DATIS.log("Pause Error: " .. tostring(err))
		end
    end

    function handler.onSimulationResume()
	    DATIS.log("Simulation Resume")

		local status, err = pcall(datis_unpause)
		if not status then
			DATIS.log("Unpause Error: " .. tostring(err))
		end
    end

	DCS.setUserCallbacks(handler)

	log.write('[DATIS]', log.INFO, 'Loaded')
end

local status, err = pcall(datis_load)
if not status then
	log.write('[DATIS]', log.INFO, "Load Error: " .. tostring(err))
end