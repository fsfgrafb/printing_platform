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
  busy: { type: Boolean, default: false }
})

defineEmits(['confirm', 'cancel', 'update:inputValue'])
</script>

<template>
  <div class="dialog-backdrop" role="presentation" @click.self="$emit('cancel')">
    <section class="confirm-dialog" role="dialog" aria-modal="true" :aria-label="title">
      <header>
        <strong>{{ title }}</strong>
        <button class="icon-button" type="button" title="关闭" :disabled="busy" @click="$emit('cancel')">
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
          @keyup.enter="$emit('confirm')"
        />
      </label>
      <footer>
        <button class="ghost-button" type="button" :disabled="busy" @click="$emit('cancel')">取消</button>
        <button
          class="primary-button"
          :class="{ 'dialog-danger-button': danger }"
          type="button"
          :disabled="busy || (inputRequired && !inputValue.trim())"
          @click="$emit('confirm')"
        >
          {{ busy ? '处理中…' : confirmText }}
        </button>
      </footer>
    </section>
  </div>
</template>
