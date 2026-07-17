<script setup>
import { onMounted, ref } from 'vue'
import { Save } from '@lucide/vue'
import { useRouter } from 'vue-router'
import { api, unwrapError } from '../api'
import { showError, showNotice, showSuccess } from '../notification'
import { refreshSession } from '../session'

const config = ref({
  daily_limit: '50',
  queue_paused: false
})
const transferStudentId = ref('')
const saving = ref(false)
const loaded = ref(false)
const router = useRouter()

onMounted(load)

async function load() {
  loaded.value = false
  try {
    const { data } = await api.get('/admin/config')
    config.value = data
    loaded.value = true
  } catch (err) {
    showError(unwrapError(err), {
      title: '系统设置加载失败',
      confirmText: '重试',
      onConfirm: load
    })
  }
}

async function save() {
  if (saving.value) return
  saving.value = true
  try {
    const dailyLimit = Math.max(0, Number.parseInt(config.value.daily_limit, 10) || 0)
    await api.put('/admin/config', { key: 'daily_limit', value: String(dailyLimit) })
    config.value.daily_limit = String(dailyLimit)
    if (transferStudentId.value.trim()) {
      await api.post('/admin/transfer', { new_admin_student_id: transferStudentId.value.trim() })
      await refreshSession()
      showNotice({
        title: '保存成功',
        message: '系统设置已保存，管理员权限已转让。',
        onConfirm: () => router.replace('/queue')
      })
      return
    }
    showSuccess('系统设置已保存。')
  } catch (err) {
    showError(unwrapError(err), { title: '保存失败' })
  } finally {
    saving.value = false
  }
}

</script>

<template>
  <section v-if="!loaded" class="page page-loading-shell">
    <p class="loading-state">正在加载系统设置</p>
  </section>

  <section v-else class="page narrow-page reveal-page">
    <header class="page-header">
      <div>
        <h1>系统设置</h1>
      </div>
    </header>

    <form class="panel form-grid" @submit.prevent="save">
      <label>
        每日限额
        <input v-model.trim="config.daily_limit" type="number" min="0" />
      </label>
      <label>
        转让管理员
        <input v-model.trim="transferStudentId" placeholder="输入已有用户学号，留空不转让" />
      </label>
      <button class="primary-button" type="submit" :disabled="saving">
        <Save :size="18" />
        <span>{{ saving ? '保存中' : '保存' }}</span>
      </button>
    </form>
  </section>
</template>
