<script setup>
import { X } from '@lucide/vue'

defineProps({
  title: { type: String, required: true },
  message: { type: String, default: '' },
  confirmText: { type: String, default: '确认' },
  danger: { type: Boolean, default: false },
  inputLabel: { type: String, default: '' },
  inputRequired: { type: Boolean, default: false },
  inputValue: { type: String, default: '' },
  showCancel: { type: Boolean, default: true },
  busy: { type: Boolean, default: false }
})

defineEmits(['confirm', 'cancel', 'update:inputValue'])
</script>

<template>
  <div class="dialog-backdrop" role="presentation" @click.self="showCancel && $emit('cancel')">
    <form class="confirm-dialog" role="dialog" aria-modal="true" :aria-label="title" @submit.prevent="$emit('confirm')">
      <header>
        <strong>{{ title }}</strong>
        <button v-if="showCancel" class="icon-button" type="button" title="关闭" :disabled="busy" @click="$emit('cancel')">
          <X :size="18" />
        </button>
      </header>
      <p v-if="message">{{ message }}</p>
      <label v-if="inputLabel" class="dialog-input">
        {{ inputLabel }}
        <input
          :value="inputValue"
          autofocus
          @input="$emit('update:inputValue', $event.target.value)"
        />
      </label>
      <footer>
        <button v-if="showCancel" class="ghost-button" type="button" :disabled="busy" @click="$emit('cancel')">取消</button>
        <button
          class="primary-button"
          :class="{ 'dialog-danger-button': danger }"
          type="submit"
          :autofocus="!inputLabel"
          :disabled="busy || (inputRequired && !inputValue.trim())"
        >
          {{ busy ? '处理中…' : confirmText }}
        </button>
      </footer>
    </form>
  </div>
</template>
