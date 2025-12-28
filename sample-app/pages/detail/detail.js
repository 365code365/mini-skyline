Page({
  data: {
    // 状态列表
    statusList: [
      { id: 1, icon: 'success', title: '操作成功', desc: '您的订单已提交成功', color: '#07C160', bgColor: '#E8F5E9' },
      { id: 2, icon: 'info', title: '温馨提示', desc: '请在30分钟内完成支付', color: '#10AEFF', bgColor: '#E3F2FD' },
      { id: 3, icon: 'warn', title: '注意事项', desc: '该操作不可撤销，请谨慎操作', color: '#F76260', bgColor: '#FFEBEE' }
    ],
    
    // 图标列表
    iconList: [
      { type: 'success', label: 'success', color: '#07C160' },
      { type: 'success_no_circle', label: 'check', color: '#07C160' },
      { type: 'info', label: 'info', color: '#10AEFF' },
      { type: 'warn', label: 'warn', color: '#F76260' },
      { type: 'waiting', label: 'waiting', color: '#10AEFF' },
      { type: 'cancel', label: 'cancel', color: '#F43530' },
      { type: 'download', label: 'download', color: '#07C160' },
      { type: 'search', label: 'search', color: '#B2B2B2' }
    ],
    
    // 卡片列表
    cardList: [
      { id: 1, icon: 'success', title: '账户设置', desc: '管理您的账户信息', color: '#07C160' },
      { id: 2, icon: 'info', title: '消息中心', desc: '查看系统通知和消息', color: '#10AEFF' },
      { id: 3, icon: 'waiting', title: '订单管理', desc: '查看和管理您的订单', color: '#FF9500' },
      { id: 4, icon: 'search', title: '帮助中心', desc: '常见问题和使用指南', color: '#8E8E93' }
    ]
  },
  
  onLoad: function() {
    console.log('📄 详情页加载完成');
  },
  
  onShow: function() {
    console.log('📄 详情页显示');
  },
  
  // 卡片点击
  onCardTap: function(e) {
    var id = e.currentTarget.dataset.id;
    var card = this.data.cardList.find(function(c) { return c.id == id; });
    if (card) {
      console.log('📌 点击卡片:', card.title);
    }
  },
  
  // 返回首页
  onBackToIndex: function() {
    console.log('🏠 返回首页');
    wx.switchTab({ url: '/pages/index/index' });
  },
  
  // 刷新
  onRefresh: function() {
    console.log('🔄 刷新页面');
  },
  
  // 清除
  onClear: function() {
    console.log('🗑️ 清除数据');
  }
});
