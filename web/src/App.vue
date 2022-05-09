<script lang="ts">
import { defineComponent } from "vue";

export default defineComponent({
  data() {
    return {
      message: "Hello World!",
      titleClass: "title",
      count: 0,
      text: "default" as string | null,
      moreText: true,
    };
  },
  methods: {
    increment() {
      // update component state
      this.count++;
    },
    // parseInput(s: Event) {
    //   this.text = s.data;
    // },
  },
});
</script>

<template>
  <h1 v-bind:class="titleClass">the message is {{ message }}</h1>
  <div v-bind:id="message"></div>
  <div :id="message"></div>
  <button v-on:click="increment">{{ count }}</button>

  <p>
    <!-- <input v-bind:value="text" v-on:input="parseInput" /> -->
    <span>{{ text }}</span>
  </p>
  <p>
    <input type="checkbox" v-model="moreText" />
    <span v-if="moreText">Display value as true - {{ moreText }}</span>
    <span v-else>Seems to be false value</span>
  </p>
</template>

<style>
.title {
  color: rgb(182, 108, 108);
}
</style>
