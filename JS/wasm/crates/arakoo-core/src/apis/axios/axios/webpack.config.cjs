const { optimize } = require('webpack');
const webpack = require('webpack');

const config = {  
  entry: './index.js',
  output: {
    path: __dirname + '/dist',
    filename: 'index.js',
    iife:false
  },
  optimization:{
    minimize:false
  }
}

module.exports = config;