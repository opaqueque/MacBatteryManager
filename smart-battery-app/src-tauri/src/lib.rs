use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// 课表配置结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduleConfig {
    /// 上课日期：0=周一, 1=周二, ..., 6=周日
    pub class_days: Vec<u8>,
    /// 上课时间，格式 HH:MM
    pub class_time: String,
    /// 提前开始充电的小时数（负数表示提前，正数表示延后）
    pub charge_start_offset: i8,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            class_days: vec![2, 3, 4], // 周三、周四、周五
            class_time: "10:10".to_string(),
            charge_start_offset: -2, // 提前2小时充电
        }
    }
}

/// 获取配置文件路径
fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_dir = PathBuf::from(home).join(".config").join("smart-battery-app");

    // 确保目录存在
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).ok();
    }

    config_dir.join("schedule.json")
}

/// 通过 osascript 提权执行 bclm 命令
#[tauri::command]
fn set_battery_limit(limit: u8) -> Result<String, String> {
    if limit != 80 && limit != 100 {
        return Err("电池限制只能是 80 或 100".to_string());
    }

    let script = format!(
        r#"do shell script "bclm write {}" with administrator privileges"#,
        limit
    );

    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if output.status.success() {
        Ok(format!("电池限制已设置为 {}%", limit))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("设置失败: {}", stderr))
    }
}

/// 保存课表配置
#[tauri::command]
fn save_schedule(config: ScheduleConfig) -> Result<String, String> {
    let config_path = get_config_path();
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    fs::write(&config_path, json)
        .map_err(|e| format!("保存配置文件失败: {}", e))?;

    Ok(format!("配置已保存到 {:?}", config_path))
}

/// 加载课表配置
#[tauri::command]
fn load_schedule() -> Result<ScheduleConfig, String> {
    let config_path = get_config_path();

    if !config_path.exists() {
        return Ok(ScheduleConfig::default());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;

    let config: ScheduleConfig = serde_json::from_str(&content)
        .map_err(|e| format!("解析配置文件失败: {}", e))?;

    Ok(config)
}

/// 获取当前电池状态
#[tauri::command]
fn get_battery_status() -> Result<String, String> {
    let output = Command::new("bclm")
        .arg("read")
        .output()
        .map_err(|e| format!("获取电池状态失败: {}", e))?;

    if output.status.success() {
        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(status)
    } else {
        // 如果 bclm 没有安装或执行失败，返回默认值
        Ok("未知".to_string())
    }
}

/// 安装并启动 LaunchDaemon 守护进程
#[tauri::command]
fn install_and_start_daemon(config_path: String) -> Result<String, String> {
    // 验证配置文件路径
    let config_path = PathBuf::from(&config_path);
    if !config_path.exists() {
        return Err("配置文件不存在".to_string());
    }

    // 读取配置以获取上课日期
    let config = load_schedule()?;
    let class_days_str = config.class_days.iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(" ");

    // 生成 scheduler.sh 脚本
    let scheduler_script = format!(r#"#!/bin/bash

# Smart Battery Manager - 定时任务脚本
# 由 LaunchDaemon 每 5 分钟执行一次

CONFIG_FILE="{}"

# 检查配置文件是否存在
if [ ! -f "$CONFIG_FILE" ]; then
    echo "$(date): 配置文件不存在: $CONFIG_FILE" >> /var/log/smart-battery.log
    exit 1
fi

# 读取配置 (使用绝对路径的 jq)
CLASS_DAYS=$({} -r '.class_days[]' "$CONFIG_FILE" 2>/dev/null)
CLASS_TIME=$({} -r '.class_time' "$CONFIG_FILE" 2>/dev/null)
CHARGE_OFFSET=$({} -r '.charge_start_offset' "$CONFIG_FILE" 2>/dev/null)

# 如果读取失败，使用默认值
if [ -z "$CLASS_TIME" ]; then
    CLASS_TIME="10:10"
fi
if [ -z "$CHARGE_OFFSET" ]; then
    CHARGE_OFFSET="-2"
fi

# 获取当前时间和日期（分钟精度）
CURRENT_MINUTES=$(( $(date +%H) * 60 + $(date +%M) ))
CURRENT_DAY=$(date +%u)  # 1=周一, 7=周日
CURRENT_DAY=$((CURRENT_DAY - 1))  # 转换为 0=周一, 6=周日

# 解析上课时间（转换为分钟）
CLASS_HOUR=$(echo "$CLASS_TIME" | cut -d: -f1)
CLASS_MINUTE=$(echo "$CLASS_TIME" | cut -d: -f2)
CLASS_MINUTES=$((10#$CLASS_HOUR * 60 + 10#$CLASS_MINUTE))

# 计算充电窗口（转换为分钟）
# CHARGE_OFFSET 是小时数，转换为分钟
OFFSET_MINUTES=$((CHARGE_OFFSET * 60))
CHARGE_START_MINUTES=$((CLASS_MINUTES + OFFSET_MINUTES))

# 检查今天是否需要充电
NEED_CHARGE=false
for day in $CLASS_DAYS; do
    if [ "$day" -eq "$CURRENT_DAY" ]; then
        NEED_CHARGE=true
        break
    fi
done

if [ "$NEED_CHARGE" = false ]; then
    # 非上课日，设置为 80%
    /opt/homebrew/bin/bclm write 80 2>/dev/null
    echo "$(date): 设置为 80% (非上课日)" >> /var/log/smart-battery.log
    exit 0
fi

# 检查是否在充电时间窗口内
# 充电窗口：CHARGE_START_MINUTES 到 CLASS_MINUTES
if [ "$CHARGE_START_MINUTES" -lt 0 ]; then
    # 负数说明是前一天的晚些时候（跨天情况，简化处理：跳过）
    /opt/homebrew/bin/bclm write 80 2>/dev/null
    echo "$(date): 设置为 80% (充电时间跨天)" >> /var/log/smart-battery.log
elif [ "$CURRENT_MINUTES" -ge "$CHARGE_START_MINUTES" ] && [ "$CURRENT_MINUTES" -lt "$CLASS_MINUTES" ]; then
    # 在充电窗口内，设置充满
    /opt/homebrew/bin/bclm write 100 2>/dev/null
    echo "$(date): 设置为 100% (充电窗口 $CHARGE_START_MINUTES-$CLASS_MINUTES)" >> /var/log/smart-battery.log
else
    # 不在充电窗口，设置 80%
    /opt/homebrew/bin/bclm write 80 2>/dev/null
    echo "$(date): 设置为 80% (当前分钟: $CURRENT_MINUTES)" >> /var/log/smart-battery.log
fi
"#, config_path.display(), "/opt/homebrew/bin/jq", "/opt/homebrew/bin/jq", "/opt/homebrew/bin/jq");

    // 生成 plist 配置文件
    let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.smartbattery.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Library/Application Support/SmartBattery/scheduler.sh</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StartInterval</key>
    <integer>300</integer>
    <key>StandardOutPath</key>
    <string>/var/log/smart-battery.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/smart-battery.log</string>
</dict>
</plist>"#;

    // 将脚本和 plist 写入 /tmp 目录
    let tmp_scheduler_path = "/tmp/scheduler.sh";
    let tmp_plist_path = "/tmp/com.smartbattery.daemon.plist";

    fs::write(tmp_scheduler_path, &scheduler_script)
        .map_err(|e| format!("写入临时脚本失败: {}", e))?;
    fs::write(tmp_plist_path, plist_content)
        .map_err(|e| format!("写入临时 plist 失败: {}", e))?;

    // 设置脚本可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(tmp_scheduler_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(tmp_scheduler_path, perms).ok();
    }

    // 构建 osascript 命令，一次性执行所有操作
    let osascript_cmd = format!(
        r#"do shell script "mkdir -p '/Library/Application Support/SmartBattery' && cp '{0}' '/Library/Application Support/SmartBattery/scheduler.sh' && cp '{1} /Library/LaunchDaemons/' && chmod +x '/Library/Application Support/SmartBattery/scheduler.sh' && launchctl load -w /Library/LaunchDaemons/com.smartbattery.daemon.plist" with administrator privileges"#,
        tmp_scheduler_path, tmp_plist_path
    );

    let output = Command::new("osascript")
        .args(["-e", &osascript_cmd])
        .output()
        .map_err(|e| format!("执行安装命令失败: {}", e))?;

    if output.status.success() {
        Ok("守护进程已安装并启动，每 5 分钟自动执行".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("安装失败: {}", stderr))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            set_battery_limit,
            save_schedule,
            load_schedule,
            get_battery_status,
            install_and_start_daemon
        ])
        .run(tauri::generate_context!())
        .expect("启动应用时发生错误");
}
