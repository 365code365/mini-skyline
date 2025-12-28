Page({
  data: {
    title: '组件展示',
    showContent: true,
    
    // 功能卡片数据
    features: [
      { id: 1, name: '数据绑定', desc: '{{}}', icon: 'success', color: '#07C160' },
      { id: 2, name: '条件渲染', desc: 'wx:if', icon: 'info', color: '#10AEFF' },
      { id: 3, name: '列表渲染', desc: 'wx:for', icon: 'waiting', color: '#10AEFF' },
      { id: 4, name: '事件处理', desc: 'bindtap', icon: 'warn', color: '#F76260' }
    ],
    
    // 图片数据
    images: [
      { id: 1, src: '/images/demo1.png', mode: 'aspectFit', label: 'aspectFit' },
      { id: 2, src: '/images/demo2.png', mode: 'aspectFill', label: 'aspectFill' }
    ],
    
    // 待办列表
    todoList: [
      { id: 1, text: '学习小程序开发', done: true },
      { id: 2, text: '完成渲染引擎', done: true },
      { id: 3, text: '添加更多组件', done: false },
      { id: 4, text: '优化性能', done: false }
    ],
    
    // 进度条数据
    progressList: [
      { id: 1, label: '下载进度', value: 30, color: '#07C160', height: 4, showInfo: false },
      { id: 2, label: '上传进度', value: 60, color: '#10AEFF', height: 6, showInfo: true },
      { id: 3, label: '安装进度', value: 90, color: '#FF6B6B', height: 8, showInfo: true }
    ],
    
    // 开关数据
    switchList: [
      { id: 1, label: '消息通知', checked: true, type: 'switch', color: '#07C160', disabled: false },
      { id: 2, label: '自动更新', checked: false, type: 'switch', color: '#07C160', disabled: false },
      { id: 3, label: '夜间模式', checked: true, type: 'checkbox', color: '#07C160', disabled: false },
      { id: 4, label: '禁用选项', checked: false, type: 'switch', color: '#07C160', disabled: true }
    ]
  },
  
  onLoad: function() {
    console.log('🏠 首页加载完成');
    console.log('📊 功能数量:', this.data.features.length);
    console.log('📝 待办数量:', this.data.todoList.length);
  },
  
  onShow: function() {
    console.log('🏠 首页显示');
  },
  
  // 点击功能卡片
  onFeatureTap: function(e) {
    var id = e.currentTarget.dataset.id;
    var feature = this.data.features.find(function(f) { return f.id == id; });
    if (feature) {
      console.log('✨ 点击功能:', feature.name);
    }
  },
  
  // 导航到表单页
  onNavigateToList: function() {
    console.log('📄 导航到表单页');
    wx.switchTab({ url: 'pages/list/list' });
  },
  
  // 导航到详情页
  onNavigateToDetail: function() {
    console.log('📄 导航到详情页');
    wx.switchTab({ url: 'pages/detail/detail' });
  },
  
  // 切换内容显示
  onToggleContent: function(e) {
    console.log('🔘 切换内容显示:', !this.data.showContent);
    this.setData({
      showContent: !this.data.showContent
    });
  }
});
