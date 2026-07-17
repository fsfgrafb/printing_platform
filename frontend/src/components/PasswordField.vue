<script setup>
import { ref } from 'vue'
import { Eye, EyeOff } from '@lucide/vue'

defineOptions({ inheritAttrs: false })

defineProps({
  modelValue: { type: String, default: '' },
  label: { type: String, required: true },
  autocomplete: { type: String, default: 'current-password' },
  required: { type: Boolean, default: false }
})

defineEmits(['update:modelValue'])

const visible = ref(false)
</script>

<template>
  <label class="password-field">
    <span>{{ label }}</span>
    <span class="password-control">
      <input
        v-bind="$attrs"
        :value="modelValue"
        :type="visible ? 'text' : 'password'"
        :autocomplete="autocomplete"
        :required="required"
        @input="$emit('update:modelValue', $event.target.value)"
      />
      <button
        class="password-toggle"
        type="button"
        :title="visible ? '隐藏密码' : '显示密码'"
        :aria-label="visible ? '隐藏密码' : '显示密码'"
        :aria-pressed="visible"
        @click="visible = !visible"
      >
        <EyeOff v-if="visible" :size="18" />
        <Eye v-else :size="18" />
      </button>
    </span>
  </label>
</template>
