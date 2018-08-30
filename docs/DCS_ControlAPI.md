<!-- TOC depthFrom:1 depthTo:6 withLinks:1 updateOnSave:1 orderedList:0 -->

- [DCS Simulation Control User Scripts](#intro)
	- [The example user script 'testGameGUI.lua':](#example-script)
- [Lua API](#lua-api)
	- [Lua File System (lfs) API](#lfs-api)
	- [DCS Control API, table 'DCS.\*'](#dcs-api)
	- [Logging API 'log.\*'](#log-api)
	- [Network specific API, available through the table 'net.'](#net-api)
	- [LuaExport API 'Export.Lo*'](#export-api)
- [The Callbacks.](#the-callbacks)
	- [Simulation Callbacks.](#sim-callbacks)
	- [GUI callbacks](#gui-callbacks)

<!-- /TOC -->

# <a name="intro"></a> DCS Simulation Control User Scripts

The behaviour of the DCS can be altered using the Lua scripts.
You define the hooks to the DCS events, and then do what you want using the provided API.

When loading, DCS searches for _Saved Games\\DCS\\Scripts\\Hooks\\\*.lua_ files,
sorts them by name and then loads into the GUI Lua-state.
Each user script is loaded into an isolated environment, so the only
thing they share is the state of the simulator.

Each script defines a set of callbacks to the DCS events and sets them with the call
    _DCS.setUserCallbacks(cb_table)_
For each callback type the hooks of all user scripts will be called in order of loading.

For callbacks which are supposed to return a value, currently there are 3 of them:

  *    _onPlayerTryConnect_
  *    _onPlayerTrySendChat_
  *    _onPlayerTryChangeSlot_

returning a value means breaking the hook call chain.
Returning nothing (or nil) means continuing the hook chain, which ends with the default allow-all handlers.

## <a name="example-script"></a> The example user script 'Hooks/test.lua':

    local test = {}

    function test.onPlayerTryConnect(ipaddr, name, ucid, playerID)
        print('onPlayerTryConnect(%s, %s, %s, %d)', ipaddr, name, ucid, playerID)
        -- if you want to gently intercept the call, allowing other user scripts to get it,
        -- you better return nothing here
        return true -- allow the player to connect
    end

    function test.onSimulationStart()
        print('Current mission is '..DCS.getMissionName())
    end

    DCS.setUserCallbacks(test)  -- here we set our callbacks

The available API is documented below.
The full list of the callbacks is at [the end of this document](#the-callbacks).

In addition, all standard lua 5.1 libraries are available as well, namely:

  *    base api, like print, etc,
  *    math.\*
  *    table.\*
  *    string.\*
  *    io.\*
  *    os.\*
  *    debug.\*

# <a name="lua-api"></a> Lua API

## <a name="lfs-api"></a> Lua File System (lfs) API

  *     _lfs.currentdir() -> string_

        Returns the path of the DCS install folder

  *     _lfs.writedir() -> string_

       Returns the path of the current 'Saved Games\DCS' folder.

  *    _lfs.tempdir() -> string_

       Returns the pat of the DCS Temp folder (AppData\Local\Temp\DCS).

  *    _lfs.mkdir()_
  *    _lfs.rmdir()_
  *    _lfs.attributes()_
  *    _lfs.dir()_
  *    _lfs.normpath()_
  *    _lfs.realpath()_

## <a name="dcs-api"></a> DCS Control API, table 'DCS.\*'

  *    _DCS.setPause(bool)_

       Pauses/resumes the simulation. Server-side only.

  *    _DCS.getPause() -> bool_

       true if simulation is paused

  *    _DCS.stopMission()_

       stops current mission

  *    _DCS.exitProcess()_

       Exits the DCS process.

  *    _DCS.isMultiplayer() -> bool_

       True when running in the multiplayer mode.

  *    _DCS.isServer() -> bool_

       True when running as a server or in the single-player mode.

  *    _DCS.getModelTime() -> number_

       returns current DCS simulation time in seconds.

  *    _DCS.getRealTime() -> number_

       returns current DCS real time in seconds relative to the DCS start time.

  *    _DCS.getMissionOptions() -> table_

      Returns the value of 'mission.options'

  *    _DCS.getMissionDescription() -> string_

      translated mission.descriptionText string

  *    _DCS.getAvailableCoalitions() -> table {_
           [coalition_id] = { name = "coalition name", }
            ...
           }

       Returns a list of coalitions which have available slots.

  *    _DCS.getAvailableSlots(coalitionID) -> array of {unitId, type, role, callsign, groupName, country}_

       Returns the list of available slots.

       NOTE: the returned unitID is actually a slotID, which for multi-seat units is 'unitID_seatID'

  *    _DCS.getCurrentMission() -> table with the currently loaded mission_

       NOTE: to get valid mission.options use _DCS.getMissionOptions()_

  *    _DCS.getMissionName() -> string_

       Returns the name of the current mission

  *    _DCS.getMissionFilename() -> string_

       Returns the file name of the current mission (returns nil when acting as a multiplayer client).

  *    _DCS.getMissionResult(string side) -> integer [0, 100]_

       Gets mission result for either 'red' or 'blue'

  *    _DCS.getUnitProperty(missionId, propertyId) -> string_

       propertyId:

           DCS.UNIT_RUNTIME_ID, // unique within runtime mission. int
           DCS.UNIT_MISSION_ID, // unique within mission file. int>0
           DCS.UNIT_NAME, // unit name, as assigned by mission designer.
           DCS.UNIT_TYPE, // unit type (Ural, ZU-23, etc)
           DCS.UNIT_CATEGORY,
           DCS.UNIT_GROUP_MISSION_ID, // group ID, unique within mission file. int>0
           DCS.UNIT_GROUPNAME, // group name, as assigned by mission designer.
           DCS.UNIT_GROUPCATEGORY,
           DCS.UNIT_CALLSIGN,
           DCS.UNIT_HIDDEN,// ME hiding
           DCS.UNIT_COALITION,// "blue", "red" or "unknown"
           DCS.UNIT_COUNTRY_ID,
           DCS.UNIT_TASK, //"unit.group.task"
           DCS.UNIT_PLAYER_NAME, // valid for network "humanable" units
           DCS.UNIT_ROLE,//"artillery_commander", "instructor", etc
           DCS.UNIT_INVISIBLE_MAP_ICON,//ME invisible map icon

  *    _DCS.getUnitType(missionId) -> typeId_

       a shortcut for DCS.getUnitProperty(missionId, DCS.UNIT_TYPE)

  *    _DCS.getUnitTypeAttribute(typeId, attr) -> string_

       Returns a value from Database: Objects[typeId][attr],

       For example:

           DCS.getUnitTypeAttribute("Ural", "DisplayName")

  *    _DCS.writeDebriefing(str)_

       Writes a custom string to the debriefing file

  *    _DCS.setUserCallbacks(cb_table)_

       Hooks the callbacks using the handlers from the provided table.

       See: "GameGUI scripts" section.

 *    _DCS.makeScreenShot(name)_
      
      Makes screenshot with given name.

## <a name="log-api"></a> Logging API 'log.\*'

Logging works as follows:

  *    each log message is accompanied with 2 attributes: a subsystem, and level.
  *    after each messages gets into the logger it passes (asynchronously) through
   a series of output filters which decide where the message will be written to.

The API is:

  *    _log.write(SUBSYSTEM_NAME, LOG_LEVEL, message, ...)_

      Sends the message to the logger. If there are any arguments after _message_,
      the actual string is formed as _string.format(message, ...)_

      SUBSYSTEM_NAME is a string

      LOG_LEVEL is one of the values, listed below

      see log.set_output() below.

  *    _log.set_output(log_file_name_wo_ext, rule_subsystem_name, rule_level_mask, rule_output_mode)_

       log_file_name_wo_ext: resulting log will be written to $WRITE_DIR/Logs/<log_file_name_wo_ext>.log

       rule_subsytem_name: the name of the subsystem whose messages to write or empty string to match all subsystems

       rule_level_mask: a sum of log-level bit flags to match messages, valid flags are:

           log.ALERT
           log.ERROR
           log.WARNING
           log.INFO
           log.DEBUG
           log.ALL - includes all of the above
           log.TRACE - a special level which is excluded from dcs.log file

       rule_output_mode: a sum of output flags:

           log.MESSAGE
           log.TIME_UTC or log.TIME_LOCAL or log.TIME_RELATIVE
           log.MODULE - this means a 'subsystem', not a DLC
           log.LEVEL
           log.FULL = log.MESSAGE + log.TIME_UTC + log.MODULE + log.LEVEL

So, in order to save net.trace(msg) messages to a file, you should issue a call:

    log.set_output('lua-net', 'LuaNET', log.TRACE, log.MESSAGE + log.TIME_UTC)

This will write to a Logs/lua-net.log file

Or, to save everything lua-network-related:

    log.set_output('lua-net', 'LuaNET', log.TRACE + log.ALL, log.MESSAGE + log.TIME_UTC + log.LEVEL)

To close the log file, you must use

    log.set_output('lua-net', '', 0, 0)

log.\* API is also available from the _Saved Games\DCS\Config\autoexec.cfg_ file so you can control log output in you local machine.


## <a name="net-api"></a> Network specific API, available through the table 'net.'

  *    _net.log(msg) -- equivalent of log.write('LuaNET', log.INFO, msg)_
  *    _net.trace(msg) -- equivalent of log.write('LuaNET', log.TRACE, msg)_

What is the difference:

  *    _log()_ always writes to __dcs.log__, but may lose messages if the output rate is too high.
  *    _trace()_ output never appears in the __dcs.log__ file, it must be explicitly directed to a log file.
It never loses messages when there's an active output, but it may block if output rate is faster than writing to the log file.

To control logger output you can use _$WRITE_DIR/Config/autoexec.cfg_ file, or call this from your network script
[(log.* API, see above)](#log-api)


  *    _net.dostring_in(state, string) -> string_

      Executes a lua-string in a given internal lua-state and returns a string result

      Valid state names are:

          'config': the state in which $INSTALL_DIR/Config/main.cfg is executed, as well as $WRITE_DIR/Config/autoexec.cfg
                    used for configuration settings
          'mission': holds current mission
          'export': runs $WRITE_DIR/Scripts/Export.lua and the relevant export API

  *    _net.send_chat(string message, bool all)_

      Send chat message. If not all, then send to my coalition (side) only.

  *    _net.send_chat_to(string message, playerID to)_

       _net.send_chat_to(string message, playerID to[, playerID from]) -- SERVER ONLY_

       Send direct chat message to a player

  *    _net.recv_chat(message[, int from=0])_

       Receive chat message locally[, pretending it was sent by another player].

       from = 0 means from the system

  *    _net.load_mission(miz_filename) -- SERVER ONLY_

       Loads a specified mission, temporarily overriding the server mission list.

  *    _net.load_next_mission() -> bool -- SERVER ONLY_

       Load the next mission from the server mission list. Returns false if list end is reached


  *    _net.get_player_list() -> array of playerID_

       Returns the list of currently connected players

  *    _net.get_my_player_id() -> playerID_

       Returns the playerID of the local player. Currently always 1 for the server.

  *    _net.get_server_id() -> playerID_

       Returns playerID of the server. Currently, always 1.

  *    _net.get_player_info(playerID) -> table_

       Returns a table of all player attributes or nil if playerID is invalid

  *    _net.get_player_info(playerID, attrName) -> value_

      Returns a value of a given attribute for the playerID.

      Currently defined attributes are:

          'id': playerID
          'name': player name
          'side': 0 - spectators, 1 - red, 2 - blue
          'slot': slotID of the player or ''
          'ping': ping of the player in ms
          'ipaddr': IP address of the player, SERVER ONLY
          'ucid': Unique Client Identifier, SERVER ONLY

  *    _net.kick(playerID, message)_

       Kick a player.

  *    _net.get_stat(playerID, statID) -> integer_

      Get statistics for player. statIDs are:

          net.PS_PING  (0) - ping (in ms)
          net.PS_CRASH (1) - number of crashes
          net.PS_CAR   (2) - number of destroyed vehicles
          net.PS_PLANE (3) - ... planes/helicopters
          net.PS_SHIP  (4) - ... ships
          net.PS_SCORE (5) - total score
          net.PS_LAND  (6) - number of landings
          net.PS_EJECT (7) - of ejects

  *    _net.get_name(playerID) -> string_

        The same as net.get_player_info(playerID, 'name')

  *    _net.get_slot(playerID) -> sideID, slotID_

       The same as:

           net.get_player_info(playerID, 'side'), net.get_player_info(playerID, 'slot')

  *    _net.set_slot(sideID, slotID)_

       Try to set the local player's slot. Empty slotID ('') puts the player into spectators.

  *    _net.force_player_slot(playerID, sideID, slotID) -> boolean_

       Forces a player to occupy a set slot. Slot '' means no slot (moves player to spectators)

       SideID: 0 - spectators, 1 - red, 2 - blue

  *    _net.set_name(playerID, name) -- OBSOLETE, works only locally_

  *    _net.lua2json(value) -> string_

       Convert a Lua value to JSON string

  *    _net.json2lua(json_string) -> value_

       Convert JSON string to a Lua value


## <a name="export-api"></a> LuaExport API 'Export.Lo*'

See _Scripts/Export.lua_ for the documentation. Note that all export
API functions are available here in the Export. namespace, not the global one.
In multiplayer the availability of the API on clients depends on the server setting.

The calls to check export capabilities:

    Export.LoIsObjectExportAllowed()  -- returns the value of server.advanced.allow_object_export
    Export.LoIsSensorExportAllowed()  -- returns the value of server.advanced.allow_sensor_export
    Export.LoIsOwnshipExportAllowed() -- returns the value of  server.advanced.allow_ownship_export


These calls are only available on clients when LoIsObjectExportAllowed() is true:

    Export.LoGetObjectById
    Export.LoGetWorldObjects


These calls are only available on clients when LoIsSensorExportAllowed() is true:

    Export.LoGetTWSInfo
    Export.LoGetTargetInformation
    Export.LoGetLockedTargetInformation
    Export.LoGetF15_TWS_Contacts
    Export.LoGetSightingSystemInfo
    Export.LoGetWingTargets


These calls are only available on clients when LoIsOwnshipExportAllowed() is true:

    Export.LoGetPlayerPlaneId
    Export.LoGetIndicatedAirSpeed
    Export.LoGetAngleOfAttack
    Export.LoGetAngleOfSideSlip
    Export.LoGetAccelerationUnits
    Export.LoGetVerticalVelocity
    Export.LoGetADIPitchBankYaw
    Export.LoGetTrueAirSpeed
    Export.LoGetAltitudeAboveSeaLevel
    Export.LoGetAltitudeAboveGroundLevel
    Export.LoGetMachNumber
    Export.LoGetRadarAltimeter
    Export.LoGetMagneticYaw
    Export.LoGetGlideDeviation
    Export.LoGetSideDeviation
    Export.LoGetSlipBallPosition
    Export.LoGetBasicAtmospherePressure
    Export.LoGetControlPanel_HSI
    Export.LoGetEngineInfo
    Export.LoGetSelfData
    Export.LoGetCameraPosition
    Export.LoSetCameraPosition
    Export.LoSetCommand
    Export.LoGetMCPState
    Export.LoGetRoute
    Export.LoGetNavigationInfo
    Export.LoGetPayloadInfo
    Export.LoGetWingInfo
    Export.LoGetMechInfo
    Export.LoGetRadioBeaconsStatus
    Export.LoGetVectorVelocity
    Export.LoGetVectorWindVelocity
    Export.LoGetSnares
    Export.LoGetAngularVelocity
    Export.LoGetHeightWithObjects
    Export.LoGetFMData


These functions are always available:

    Export.LoGetPilotName
    Export.LoGetAltitude
    Export.LoGetNameByType
    Export.LoGeoCoordinatesToLoCoordinates
    Export.LoCoordinatesToGeoCoordinates
    Export.LoGetVersionInfo
    Export.LoGetWindAtPoint
    Export.LoGetModelTime
    Export.LoGetMissionStartTime


These are not available in the hooks:

    --Export.LoSetSharedTexture
    --Export.LoRemoveSharedTexture
    --Export.LoUpdateSharedTexture

# <a name="the-callbacks"></a> The Callbacks.
## <a name="sim-callbacks"></a> Simulation Callbacks.

    function onMissionLoadBegin()
    end

    function onMissionLoadProgress(progress, message)
    end

    function onMissionLoadEnd()
    end

    function onSimulationStart()
    end

    function onSimulationStop()
    end

    function onSimulationFrame()
    end

    function onSimulationPause()
    end

    function onSimulationResume()
    end

    function onGameEvent(eventName,arg1,arg2,arg3,arg4)
        --"friendly_fire", playerID, weaponName, victimPlayerID
        --"mission_end", winner, msg
        --"kill", killerPlayerID, killerUnitType, killerSide, victimPlayerID, victimUnitType, victimSide, weaponName
        --"self_kill", playerID
        --"change_slot", playerID, slotID, prevSide
        --"connect", playerID, name
        --"disconnect", playerID, name, playerSide, reason_code
        --"crash", playerID, unit_missionID
        --"eject", playerID, unit_missionID
        --"takeoff", playerID, unit_missionID, airdromeName
        --"landing", playerID, unit_missionID, airdromeName
        --"pilot_death", playerID, unit_missionID
    end

    function onNetConnect(localPlayerID)
    end

    function onNetMissionChanged(newMissionName)
    end

    function onNetDisconnect(reason_msg, err_code)
    end

    -- disconnect reason codes:
      net.ERR_INVALID_ADDRESS
      net.ERR_CONNECT_FAILED
      net.ERR_WRONG_VERSION
      net.ERR_PROTOCOL_ERROR
      net.ERR_TAINTED_CLIENT
      net.ERR_INVALID_PASSWORD
      net.ERR_BANNED
      net.ERR_BAD_CALLSIGN

      net.ERR_TIMEOUT
      net.ERR_KICKED


    function onPlayerConnect(id)
    end

    function onPlayerDisconnect(id, err_code)
        -- this is never called for local playerID
    end

    function onPlayerStart(id)
        -- a player entered the simulation
        -- this is never called for local playerID
    end

    function onPlayerStop(id)
        -- a player left the simulation (happens right before a disconnect, if player exited by desire)
        -- this is never called for local playerID
    end

    function onPlayerChangeSlot(id)
        -- a player successfully changed the slot
        -- this will also come as onGameEvent('change_slot', playerID, slotID),
        -- if allowed by server.advanced.event_Connect setting
    end


    --- These 3 functions are different from the rest:
    --- 1. they are called directly from the network code, so try to make them as fast as possible
    --- 2. they return a result
    -- The code shows the default implementations.

    function onPlayerTryConnect(addr, name, ucid, playerID) --> true | false, "disconnect reason"
        return true
    end

    function onPlayerTrySendChat(playerID, msg, all) -- -> filteredMessage | "" - empty string drops the message
        return msg
    end

    function onPlayerTryChangeSlot(playerID, side, slotID) -- -> true | false
        return true
    end

## <a name="gui-callbacks"></a> GUI Callbacks.

    function onChatMessage(message, from)
        -- this one may be useful for chat archiving
    end

    function onShowRadioMenu(a_h)
    end

    function onShowPool()
    end

    function onShowGameMenu()
    end

    function onShowBriefing()
    end

    function onShowChatAll()
    end

    function onShowChatTeam()
    end

    function onShowChatRead()
    end

    function onShowMessage(a_text, a_duration)
    end

    function onTriggerMessage(message, duration, clearView)
    end

    function onRadioMessage(message, duration)
    end

    function onRadioCommand(command_message)
    end

---------------------------------------------------------------------------------------------------

Happy hacking!

Sincerely,
dsb at eagle dot ru

[To the top.](#intro)