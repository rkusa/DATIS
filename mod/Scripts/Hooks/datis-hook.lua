package.cpath = package.cpath..";"..lfs.writedir().."Mods\\tech\\DATIS\\bin\\?.dll;"

local datis = nil

function datis_start()
	if not DCS.isServer() then
		log.write("[DATIS]", log.INFO, "Starting DATIS skipped for not being the Server ...")
		return
	end

	log.write("[DATIS]", log.INFO, "Starting ...")

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
	log.write("[DATIS]", log.INFO, "Loading ...")

	local handler = {}

	function handler.onSimulationStop()
		log.write("[DATIS]", log.INFO, "Stopping")

		local status, err = pcall(datis_stop)
		if not status then
			log.write("[DATIS]", log.INFO, "Stop Error: " .. tostring(err))
		end
	end

	function handler.onSimulationPause()
		log.write("[DATIS]", log.INFO, "Pausing")

		local status, err = pcall(datis_pause)
		if not status then
			log.write("[DATIS]", log.INFO, "Pause Error: " .. tostring(err))
		end
	end

	function handler.onSimulationResume()
		log.write("[DATIS]", log.INFO, "Resuming")

		if datis == nil then
			local status, err = pcall(datis_start)
			if not status then
				log.write("[DATIS]", log.INFO, "Start Error: " .. tostring(err))
			end
		else
			local status, err = pcall(datis_unpause)
			if not status then
				log.write("[DATIS]", log.INFO, "Unpause Error: " .. tostring(err))
			end
		end
		
	end

	DCS.setUserCallbacks(handler)

	log.write("[DATIS]", log.INFO, "Loaded")
end

local status, err = pcall(datis_load)
if not status then
	log.write("[DATIS]", log.INFO, "Load Error: " .. tostring(err))
end