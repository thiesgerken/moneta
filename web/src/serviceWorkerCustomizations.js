/* eslint-disable no-restricted-globals */

// self.addEventListener('install', e => {
//   console.log('workbox "install" event fired');
//   console.log(e);

//   self.skipWaiting();
// });

// self.addEventListener('beforeinstallprompt', e => {
//   console.log('workbox "beforeinstallprompt" event fired');
//   console.log(e);
// });

// eslint-disable-next-line no-unused-vars
self.addEventListener('fetch', e => {
  // this needs to be here, even if it does not do anything.
  console.log('[Service Worker] fetch event fired');
  console.log(e);
});

self.addEventListener('push', e => {
  const data = e.data.json();

  console.log(`[Service Worker] Push received"`);
  console.log(data);

  const title = 'Moneta';
  const options = {
    body: JSON.stringify(data),
    icon: 'public/icon-512.png',
    // badge: 'images/badge.png'
  };

  e.waitUntil(self.registration.showNotification(title, options));
});
