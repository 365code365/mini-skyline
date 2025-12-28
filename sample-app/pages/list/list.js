Page({
  data: {
    title: '表单组件',
    volume: 50,
    brightness: 70,
    progress: 30,
    fruits: ['apple', 'banana'],
    gender: 'male'
  },
  
  onLoad: function() {
    console.log('📝 表单页面加载完成');
  },
  
  onShow: function() {
    console.log('📝 表单页面显示');
  },
  
  onVolumeChange: function(e) {
    console.log('🔊 音量:', e.detail.value);
    this.setData({ volume: e.detail.value });
  },
  
  onBrightnessChange: function(e) {
    console.log('☀️ 亮度:', e.detail.value);
    this.setData({ brightness: e.detail.value });
  },
  
  onCheckboxChange: function(e) {
    console.log('☑️ 选中的水果:', e.detail.value);
    this.setData({ fruits: e.detail.value });
  },
  
  onRadioChange: function(e) {
    console.log('🔘 选择的性别:', e.detail.value);
    this.setData({ gender: e.detail.value });
  },
  
  onInputChange: function(e) {
    console.log('📝 输入内容:', e.detail.value);
  }
});
