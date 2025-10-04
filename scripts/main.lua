-- SavePoint Teleport Mod - File-based Communication
-- 使用文件进行 Tauri 和 Lua 之间的通信

print("[SavePointTeleport] Starting initialization...")

--------------------------------
-- 1) 配置
--------------------------------
local CFG = {
  save_class = 'BP_SavePoint_C',
  excludes   = { 'Chair', 'Nomesh' },
  poll_ms    = 100,
}

-- Get system temp directory with project subdirectory
local function get_project_dir()
  local SEP = package.config:sub(1,1)
  local temp_dir

  if SEP == '\\' then
    -- Windows
    temp_dir = os.getenv('TEMP') or os.getenv('TMP') or 'C:\\Windows\\Temp'
  else
    -- Unix-like
    temp_dir = os.getenv('TMPDIR') or '/tmp'
  end

  local project_dir = temp_dir .. (SEP == '\\' and '\\' or '/') .. 'Forged-In-Shadow-Torch-Save-Point-Teleport-Mod'
  return project_dir
end

local project_dir = get_project_dir()
local SEP = package.config:sub(1,1)
CFG.cmd_file = project_dir .. (SEP == '\\' and '\\' or '/') .. 'cmd.txt'
CFG.resp_file = project_dir .. (SEP == '\\' and '\\' or '/') .. 'resp.txt'

print("[SavePointTeleport] Project dir: " .. project_dir)
print("[SavePointTeleport] Command file: " .. CFG.cmd_file)
print("[SavePointTeleport] Response file: " .. CFG.resp_file)

--------------------------------
-- 2) 状态
--------------------------------
local S = {
  save_points = {},
}

--------------------------------
-- 3) JSON 工具
--------------------------------
local function json_escape(s)
  s = tostring(s):gsub('\\','\\\\'):gsub('"','\\"')
  s = s:gsub('\r','\\r'):gsub('\n','\\n')
  return s
end

local function encode_scan_result()
  local t = {}
  t[#t+1] = '{"save_points":['
  for i,p in ipairs(S.save_points) do
    if i>1 then t[#t+1]=',' end
    t[#t+1] = '{"i":' .. tostring(i) .. ',"name":"' .. json_escape(p.name) ..
              '","x":' .. tostring(p.location.X) ..
              ',"y":' .. tostring(p.location.Y) ..
              ',"z":' .. tostring(p.location.Z) .. '}'
  end
  t[#t+1] = ']}'
  return table.concat(t)
end

--------------------------------
-- 4) UE 功能
--------------------------------
local function scan_save_points()
  print("[SavePointTeleport] Scanning save points...")
  S.save_points = {}
  local actors = FindAllOf and FindAllOf("Actor")
  if not actors then
    print("[SavePointTeleport] ERROR: FindAllOf returned nil")
    return "ERR no actors"
  end

  print("[SavePointTeleport] Found " .. tostring(#actors) .. " actors")

  for _, actor in ipairs(actors) do
    if actor and actor.IsValid and actor:IsValid() then
      local fullname = actor.GetFullName and actor:GetFullName() or ''
      if fullname:find(CFG.save_class, 1, true) then
        local excluded = false
        for _,k in ipairs(CFG.excludes) do
          if fullname:find(k,1,true) then excluded=true break end
        end
        if not excluded then
          local name = fullname:match('%.([^.]*)$') or CFG.save_class
          local loc = actor.K2_GetActorLocation and actor:K2_GetActorLocation() or nil
          if loc and loc.X and loc.Y and loc.Z then
            S.save_points[#S.save_points+1] = {name=name, location={X=loc.X, Y=loc.Y, Z=loc.Z}}
            print("[SavePointTeleport] Found: " .. name)
          end
        end
      end
    end
  end

  table.sort(S.save_points, function(a,b) return tostring(a.name) < tostring(b.name) end)
  print("[SavePointTeleport] Scan complete: " .. tostring(#S.save_points) .. " points")
  return "OK " .. tostring(#S.save_points)
end

local function with_pawn(f)
  ExecuteInGameThread(function()
    local pc = FindFirstOf and FindFirstOf("PlayerController")
    local pawn = pc and pc.K2_GetPawn and pc:K2_GetPawn() or nil
    if pawn then f(pawn) end
  end)
end

local function tp_index(idx)
  if idx < 1 or idx > #S.save_points then return "ERR index out of range" end
  local target = S.save_points[idx]
  print("[SavePointTeleport] Teleporting to: " .. target.name)
  with_pawn(function(pawn)
    local out_hit = {}
    pawn:K2_SetActorLocation(target.location, false, out_hit, true)
  end)
  return "OK teleported"
end

local function tp_xyz(x,y,z)
  print("[SavePointTeleport] Teleporting to XYZ")
  with_pawn(function(pawn)
    local out_hit = {}
    pawn:K2_SetActorLocation({X=x,Y=y,Z=z}, false, out_hit, true)
  end)
  return "OK teleported"
end

--------------------------------
-- 5) 命令处理
--------------------------------
local function handle_cmd(line)
  print("[SavePointTeleport] Command: " .. line)

  -- Extract command and timestamp
  local cmd, timestamp = line:match('^%s*(%S+)%s*(%S*)')
  if not cmd then return "ERR empty" end
  cmd = cmd:upper()

  -- Build timestamp suffix for response
  local timestamp_suffix = ""
  if timestamp and timestamp ~= "" and tonumber(timestamp) then
    timestamp_suffix = " TIMESTAMP:" .. timestamp
    print("[SavePointTeleport] Command timestamp: " .. timestamp)
  end

  if cmd == 'PING' then
    return "PONG" .. timestamp_suffix
  elseif cmd == 'SCAN' then
    local res = scan_save_points()
    if res:sub(1,2) == "OK" then
      return encode_scan_result() .. timestamp_suffix
    else
      return res .. timestamp_suffix
    end
  elseif cmd == 'TP' then
    local idx = tonumber(line:match('^%s*%S+%s+(%d+)'))
    if not idx then return "ERR usage: TP <index>" .. timestamp_suffix end
    return tp_index(idx) .. timestamp_suffix
  elseif cmd == 'TPNAME' then
    local name = line:match('^%s*%S+%s+(%S+)')
    if not name then return "ERR usage: TPNAME <name>" .. timestamp_suffix end

    -- Find savepoint by name (exact match or substring)
    local found = nil
    for i, p in ipairs(S.save_points) do
      if p.name == name then
        found = i
        break
      end
    end

    -- If no exact match, try substring match
    if not found then
      for i, p in ipairs(S.save_points) do
        if p.name:find(name, 1, true) then
          found = i
          break
        end
      end
    end

    if not found then
      return "ERR savepoint not found: " .. name .. timestamp_suffix
    end

    return tp_index(found) .. timestamp_suffix
  elseif cmd == 'MOVE' then
    local parts = {}
    for num in line:gmatch('[%-%.%d]+') do
      parts[#parts+1] = tonumber(num)
    end
    if #parts >= 3 then
      return tp_xyz(parts[1], parts[2], parts[3]) .. timestamp_suffix
    else
      return "ERR usage: MOVE <x> <y> <z>" .. timestamp_suffix
    end
  end

  return "ERR unknown command" .. timestamp_suffix
end

--------------------------------
-- 6) 文件通信
--------------------------------
local function file_exists(path)
  local f = io.open(path, "r")
  if f then
    f:close()
    return true
  end
  return false
end

local function read_file(path)
  local f = io.open(path, "r")
  if not f then return nil end
  local content = f:read("*a")
  f:close()
  return content
end

local function write_file(path, content)
  local f = io.open(path, "w")
  if not f then return false end
  f:write(content)
  f:close()
  return true
end

local function delete_file(path)
  os.remove(path)
end

-- Create project directory
local function ensure_project_dir()
  local SEP = package.config:sub(1,1)
  local project_dir = get_project_dir()

  -- Try to create directory using os.execute
  if SEP == '\\' then
    -- Windows
    os.execute('mkdir "' .. project_dir .. '" 2>nul')
  else
    -- Unix-like
    os.execute('mkdir -p "' .. project_dir .. '"')
  end
end

local function poll_once()
  if file_exists(CFG.cmd_file) then
    print("[SavePointTeleport] Command file detected!")
    local cmd = read_file(CFG.cmd_file)
    if cmd then
      print("[SavePointTeleport] Command read: " .. cmd)
      delete_file(CFG.cmd_file)
      print("[SavePointTeleport] Command file deleted")
      local resp = handle_cmd(cmd)
      print("[SavePointTeleport] Writing response...")
      local success = write_file(CFG.resp_file, resp)
      if success then
        print("[SavePointTeleport] Response written successfully")
      else
        print("[SavePointTeleport] ERROR: Failed to write response file")
      end
    else
      print("[SavePointTeleport] ERROR: Failed to read command file")
    end
  end
end

--------------------------------
-- 7) 启动
--------------------------------
print("[SavePointTeleport] Starting file polling...")

-- Ensure project directory exists
ensure_project_dir()
print("[SavePointTeleport] Project directory created/verified")

if type(LoopAsync) ~= "function" then
  print("[SavePointTeleport] ERROR: LoopAsync not available")
  return
end

LoopAsync(CFG.poll_ms, function()
  poll_once()
  return false
end)

print("[SavePointTeleport] Initialization complete!")

-- Console command
if type(RegisterConsoleCommandHandler) == "function" then
  RegisterConsoleCommandHandler("savetp", function(_, param)
    if param == "scan" then
      print(scan_save_points())
    else
      print("[SavePointTeleport] Points: " .. tostring(#S.save_points))
    end
  end)
  print("[SavePointTeleport] Console command 'savetp' registered")
end
