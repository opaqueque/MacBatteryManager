<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ScheduleConfig } from './types'

// 当前电池状态
const batteryStatus = ref('未知')
const loading = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

// 课表配置
const schedule = ref<ScheduleConfig>({
  class_days: [2, 3, 4],
  class_time: '10:10',
  charge_start_offset: -2
})

// 星期选项
const weekDays = [
  { value: 0, label: '周一' },
  { value: 1, label: '周二' },
  { value: 2, label: '周三' },
  { value: 3, label: '周四' },
  { value: 4, label: '周五' },
  { value: 5, label: '周六' },
  { value: 6, label: '周日' }
]

// 切换星期选中状态
function toggleDay(day: number) {
  const index = schedule.value.class_days.indexOf(day)
  if (index === -1) {
    schedule.value.class_days.push(day)
  } else {
    schedule.value.class_days.splice(index, 1)
  }
  schedule.value.class_days.sort()
}

// 显示消息
function showMessage(text: string, type: 'success' | 'error' = 'success') {
  message.value = text
  messageType.value = type
  setTimeout(() => {
    message.value = ''
  }, 3000)
}

// 设置电池限制
async function setLimit(limit: number) {
  loading.value = true
  try {
    const result = await invoke<string>('set_battery_limit', { limit })
    showMessage(result)
    await getBatteryStatus()
  } catch (error) {
    showMessage(String(error), 'error')
  } finally {
    loading.value = false
  }
}

// 获取电池状态
async function getBatteryStatus() {
  try {
    batteryStatus.value = await invoke<string>('get_battery_status')
  } catch {
    batteryStatus.value = '未知'
  }
}

// 保存配置
async function saveSchedule() {
  loading.value = true
  try {
    await invoke('save_schedule', { config: schedule.value })
    showMessage('课表配置已保存')
  } catch (error) {
    showMessage(String(error), 'error')
  } finally {
    loading.value = false
  }
}

// 加载配置
async function loadSchedule() {
  try {
    const config = await invoke<ScheduleConfig>('load_schedule')
    schedule.value = config
  } catch (error) {
    console.error('加载配置失败:', error)
  }
}

// 生成定时脚本
async function generateScript() {
  loading.value = true
  try {
    const result = await invoke<string>('generate_scheduler_script')
    showMessage(result)
  } catch (error) {
    showMessage(String(error), 'error')
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await loadSchedule()
  await getBatteryStatus()
})
</script>

<template>
  <div class="min-h-screen bg-gray-50 p-6">
    <div class="max-w-sm mx-auto">
      <!-- 标题 -->
      <div class="text-center mb-6">
        <h1 class="text-2xl font-semibold text-gray-900">Smart Battery</h1>
        <p class="text-sm text-gray-500 mt-1">电池智能管理</p>
      </div>

      <!-- 当前状态卡片 -->
      <div class="bg-white rounded-2xl shadow-sm border border-gray-100 p-5 mb-4">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm text-gray-500">当前电池限制</p>
            <p class="text-4xl font-medium text-gray-900 mt-1">{{ batteryStatus }}</p>
          </div>
          <div class="w-16 h-16 rounded-full bg-gradient-to-br from-blue-50 to-blue-100 flex items-center justify-center">
            <svg class="w-8 h-8 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
            </svg>
          </div>
        </div>
      </div>

      <!-- 快速操作 -->
      <div class="grid grid-cols-2 gap-3 mb-6">
        <button
          @click="setLimit(80)"
          :disabled="loading"
          class="bg-white hover:bg-gray-50 active:bg-gray-100 transition-colors rounded-xl border border-gray-200 p-4 flex flex-col items-center disabled:opacity-50"
        >
          <div class="w-10 h-10 rounded-full bg-orange-100 flex items-center justify-center mb-2">
            <svg class="w-5 h-5 text-orange-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
            </svg>
          </div>
          <span class="text-sm font-medium text-gray-700">限制 80%</span>
          <span class="text-xs text-gray-400 mt-0.5">日常使用</span>
        </button>

        <button
          @click="setLimit(100)"
          :disabled="loading"
          class="bg-white hover:bg-gray-50 active:bg-gray-100 transition-colors rounded-xl border border-gray-200 p-4 flex flex-col items-center disabled:opacity-50"
        >
          <div class="w-10 h-10 rounded-full bg-green-100 flex items-center justify-center mb-2">
            <svg class="w-5 h-5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
            </svg>
          </div>
          <span class="text-sm font-medium text-gray-700">充满 100%</span>
          <span class="text-xs text-gray-400 mt-0.5">上课前</span>
        </button>
      </div>

      <!-- 课表设置 -->
      <div class="bg-white rounded-2xl shadow-sm border border-gray-100 p-5">
        <h2 class="text-base font-medium text-gray-900 mb-4">课表设置</h2>

        <!-- 上课日期 -->
        <div class="mb-4">
          <label class="text-sm text-gray-500 mb-2 block">上课日期</label>
          <div class="flex gap-1.5">
            <button
              v-for="day in weekDays"
              :key="day.value"
              @click="toggleDay(day.value)"
              :class="[
                'flex-1 py-2 rounded-lg text-xs font-medium transition-colors',
                schedule.class_days.includes(day.value)
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
              ]"
            >
              {{ day.label.slice(1) }}
            </button>
          </div>
        </div>

        <!-- 上课时间 -->
        <div class="mb-4">
          <label class="text-sm text-gray-500 mb-2 block">上课时间</label>
          <input
            v-model="schedule.class_time"
            type="time"
            class="w-full px-4 py-2.5 rounded-lg border border-gray-200 text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
        </div>

        <!-- 充电提前时间 -->
        <div class="mb-5">
          <label class="text-sm text-gray-500 mb-2 block">
            提前充电时间
            <span class="text-gray-400">(小时)</span>
          </label>
          <select
            v-model.number="schedule.charge_start_offset"
            class="w-full px-4 py-2.5 rounded-lg border border-gray-200 text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white"
          >
            <option :value="-3">提前 3 小时</option>
            <option :value="-2">提前 2 小时</option>
            <option :value="-1">提前 1 小时</option>
            <option :value="0">整点开始</option>
          </select>
        </div>

        <!-- 操作按钮 -->
        <div class="flex gap-2">
          <button
            @click="saveSchedule"
            :disabled="loading"
            class="flex-1 bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white py-2.5 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          >
            保存配置
          </button>
          <button
            @click="generateScript"
            :disabled="loading"
            class="px-4 bg-gray-100 hover:bg-gray-200 active:bg-gray-300 text-gray-700 py-2.5 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          >
            生成脚本
          </button>
        </div>
      </div>

      <!-- 消息提示 -->
      <transition
        enter-active-class="transition ease-out duration-200"
        enter-from-class="opacity-0 translate-y-2"
        enter-to-class="opacity-100 translate-y-0"
        leave-active-class="transition ease-in duration-150"
        leave-from-class="opacity-100 translate-y-0"
        leave-to-class="opacity-0 translate-y-2"
      >
        <div
          v-if="message"
          :class="[
            'fixed bottom-6 left-1/2 -translate-x-1/2 px-4 py-2.5 rounded-lg shadow-lg text-sm font-medium',
            messageType === 'success' ? 'bg-gray-900 text-white' : 'bg-red-500 text-white'
          ]"
        >
          {{ message }}
        </div>
      </transition>
    </div>
  </div>
</template>
