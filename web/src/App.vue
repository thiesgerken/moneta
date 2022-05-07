<script>
export default {
  data() {
    return {
      message: "Hello World!",
      titleClass: "title",
    };
  },
};
</script>

<template>
  <h1 v-bind:class="titleClass">the message is {{ message }}</h1>
  <div v-bind:id="message"></div>
  <div :id="message"></div>
</template>

<style>
.title {
  color: rgb(182, 108, 108);
}
</style>
