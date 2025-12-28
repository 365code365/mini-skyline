Page({
  data: {
    title: '更多组件'
  },
  
  onLoad: function(options) {
    console.log('📦 详情页面加载完成', options);
  },
  
  onShow: function() {
    console.log('📦 详情页面显示');
  },
  
  onCardTap: function(e) {
    var id = e.currentTarget.dataset.id;
    console.log('🎯 点击卡片:', id);
  },
  
  onSubmit: function() {
    console.log('✅ 确认提交');
  },
  
  onCancel: function() {
    console.log('❌ 取消操作');
  },
  
  onSave: function() {
    console.log('💾 保存');
  },
  
  onShare: function() {
    console.log('📤 分享');
  },
  
  onDelete: function() {
    console.log('🗑️ 删除');
  }
});
