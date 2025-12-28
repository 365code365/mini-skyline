Page({
  data: {
    title: '组件展示',
    switchValue: true
  },
  
  onLoad: function() {
    console.log('🏠 首页加载完成');
  },
  
  onShow: function() {
    console.log('🏠 首页显示');
  },
  
  onPrimaryTap: function() {
    console.log('✅ 点击了主要按钮');
  },
  
  onDefaultTap: function() {
    console.log('📝 点击了默认按钮');
  },
  
  onWarnTap: function() {
    console.log('⚠️ 点击了警告按钮');
  },
  
  onSwitchChange: function(e) {
    console.log('🔘 开关状态变化:', e.detail.value);
    this.setData({
      switchValue: e.detail.value
    });
  }
});
