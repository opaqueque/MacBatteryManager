#!/bin/bash

# Smart Battery Manager - 后台定时任务脚本
# ==========================================
# 使用方法:
# 1. 将脚本路径添加到 crontab: crontab -e
# 2. 添加定时任务，例如每小时执行一次:
#    0 * * * * /Users/changchen/working_place/MacBatteryManager/smart-battery-app/background_task.sh
#
# 或使用 launchd 创建定时任务

# 配置文件路径
CONFIG_FILE="$HOME/.config/smart-battery-app/schedule.json"

# 日志文件
LOG_FILE="$HOME/.config/smart-battery-app/battery-scheduler.log"

# 检查配置文件是否存在
if [ ! -f "$CONFIG_FILE" ]; then
    echo "$(date): 配置文件不存在: $CONFIG_FILE" >> "$LOG_FILE"
    exit 1
fi

# 读取配置 (使用 jq)
CLASS_DAYS=$(jq -r '.class_days[]' "$CONFIG_FILE" 2>/dev/null)
CLASS_TIME=$(jq -r '.class_time' "$CONFIG_FILE" 2>/dev/null)
CHARGE_OFFSET=$(jq -r '.charge_start_offset' "$CONFIG_FILE" 2>/dev/null)

# 如果读取失败，使用默认值
if [ -z "$CLASS_TIME" ]; then
    CLASS_TIME="10:10"
fi
if [ -z "$CHARGE_OFFSET" ]; then
    CHARGE_OFFSET="-2"
fi

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

# 处理负数情况（比如 -2 小时变成前一天）
if [ "$CHARGE_START_HOUR" -lt 0 ]; then
    CHARGE_START_HOUR=$((24 + CHARGE_START_HOUR))
    # 如果是负数，说明是前一天的这个时候，需要检查是否过了午夜
    # 这里简化为只检查小时
fi

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
    osascript -e 'do shell script "bclm write 80" with administrator privileges' 2>/dev/null
    echo "$(date): 设置为 80% (非上课日，当前是周$CURRENT_DAY)" >> "$LOG_FILE"
    exit 0
fi

# 检查是否在充电时间窗口内
# 充电窗口：从 CHARGE_OFFSET 小时后到上课时间
# 例如：上课时间 10:10，提前 2 小时，则充电窗口为 08:10 - 10:10

# 判断当前小时是否在充电窗口内
if [ "$CURRENT_HOUR" -ge "$CHARGE_START_HOUR" ] && [ "$CURRENT_HOUR" -lt "$CLASS_HOUR" ]; then
    # 在充电窗口内，设置充满
    osascript -e 'do shell script "bclm write 100" with administrator privileges' 2>/dev/null
    echo "$(date): 设置为 100% (充电窗口 $CHARGE_START_HOUR:$CLASS_MINUTE - $CLASS_HOUR:$CLASS_MINUTE)" >> "$LOG_FILE"
else
    # 不在充电窗口，设置 80%
    osascript -e 'do shell script "bclm write 80" with administrator privileges' 2>/dev/null
    echo "$(date): 设置为 80% (当前时间 $CURRENT_HOUR:$CURRENT_MINUTE 不在充电窗口内)" >> "$LOG_FILE"
fi
