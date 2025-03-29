'use strict';

chrome.devtools.panels.create(
    'Svelte Chrome Extension Starter - Devtools Panel',
    'images/icon-128x128.png',
    'pages/devtools-panel/index.html',
    (_panel) => {
        // code invoked on panel creation
    },
);
