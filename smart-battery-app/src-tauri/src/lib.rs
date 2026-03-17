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

/// 生成后台定时任务脚本
#[tauri::command]
fn generate_scheduler_script() -> Result<String, String> {
    let config = load_schedule()?;
    let config_path = get_config_path();

    let script = format!(r#"#!/bin/bash

# Smart Battery Manager - 定时任务脚本
# 将此脚本添加到 crontab 或 launchd 中定期执行

CONFIG_FILE="{}"

# 检查配置文件是否存在
if [ ! -f "$CONFIG_FILE" ]; then
    echo "配置文件不存在: $CONFIG_FILE"
    exit 1
fi

# 读取配置
CLASS_DAYS=$(jq -r '.class_days[]' "$CONFIG_FILE" 2>/dev/null)
CLASS_TIME=$(jq -r '.class_time' "$CONFIG_FILE" 2>/dev/null)
CHARGE_OFFSET=$(jq -r '.charge_start_offset' "$CONFIG_FILE" 2>/dev/null)

# 获取当前时间和日期
CURRENT_HOUR=$(date +%H)
CURRENT_MINUTE=$(date +%M)
CURRENT_DAY=$(date +%u)  # 1=周一, 7=周日
CURRENT_DAY=$((CURRENT_DAY - 1))  # 转换为 0=周一, 6=周日

# 解析上课时间
CLASS_HOUR=$(echo "$CLASS_TIME" | cut -d: -f1)
CLASS_MINUTE=$(echo "$CLASS_TIME" | cut -d: -f2)

# 计算充电开始时间
CHARGE_START_HOUR=$((CLASS_HOUR + CHARGE_OFFSET))

# 检查今天是否需要充电
NEED_CHARGE=false
for day in {}; do
    if [ "$day" -eq "$CURRENT_DAY" ]; then
        NEED_CHARGE=true
        break
    fi
done

if [ "$NEED_CHARGE" = false ]; then
    # 非上课日，设置为 80%
    osascript -e 'do shell script "bclm write 80" with administrator privileges'
    echo "$(date): 设置为 80% (非上课日)"
    exit 0
fi

# 检查是否在充电时间窗口内
# 充电窗口：提前 CHARGE_OFFSET 小时到上课时间
if [ "$CHARGE_START_HOUR" -lt 0 ]; then
    CHARGE_START_HOUR=$((24 + CHARGE_START_HOUR))
fi

# 判断是否需要充满
if [ "$CURRENT_HOUR" -ge "$CHARGE_START_HOUR" ] && [ "$CURRENT_HOUR" -lt "$CLASS_HOUR" ]; then
    # 在充电窗口内，设置充满
    osascript -e 'do shell script "bclm write 100" with administrator privileges'
    echo "$(date): 设置为 100% (充电窗口)"
else
    # 不在充电窗口，设置 80%
    osascript -e 'do shell script "bclm write 80" with administrator privileges'
    echo "$(date): 设置为 80%"
fi
"#, config_path.display(), config.class_days.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(" "));

    // 保存脚本到配置目录
    let script_path = get_config_path()
        .parent()
        .unwrap_or(&PathBuf::from("."))
        .join("battery-scheduler.sh");

    fs::write(&script_path, &script)
        .map_err(|e| format!("保存脚本失败: {}", e))?;

    // 设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).ok();
    }

    Ok(format!("脚本已生成到: {:?}", script_path))
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
            generate_scheduler_script
        ])
        .run(tauri::generate_context!())
        .expect("启动应用时发生错误");
}
