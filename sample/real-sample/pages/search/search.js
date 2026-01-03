const app = getApp();

Page({
  data: {
    keyword: '',
    hasSearched: false,
    searchHistory: [],
    hotKeywords: [
      '无线蓝牙耳机',
      '智能手机',
      '机械键盘',
      '显示器',
      '笔记本',
      '路由器',
      '充电宝',
      '蓝牙音箱'
    ],
    results: []
  },

  onLoad: function(options) {
    this.loadSearchHistory();
  },

  // 加载搜索历史
  loadSearchHistory: function() {
    try {
      const history = wx.getStorageSync('searchHistory') || [];
      this.setData({ searchHistory: history });
    } catch (e) {
      console.error('加载搜索历史失败', e);
    }
  },

  // 保存搜索历史
  saveSearchHistory: function(keyword) {
    let history = this.data.searchHistory;
    // 移除已存在的相同关键词
    history = history.filter(item => item !== keyword);
    // 添加到开头
    history.unshift(keyword);
    // 最多保存10条
    if (history.length > 10) {
      history = history.slice(0, 10);
    }

    this.setData({ searchHistory: history });
    try {
      wx.setStorageSync('searchHistory', history);
    } catch (e) {
      console.error('保存搜索历史失败', e);
    }
  },

  // 输入
  onInput: function(e) {
    this.setData({ keyword: e.detail.value });
  },

  // 搜索
  onSearch: function() {
    const keyword = this.data.keyword.trim();
    if (!keyword) {
      wx.showToast({ title: '请输入搜索内容', icon: 'none' });
      return;
    }

    this.saveSearchHistory(keyword);
    this.performSearch(keyword);
  },

  // 执行搜索
  performSearch: function(keyword) {
    this.setData({ hasSearched: true });

    // 模拟搜索结果
    const allProducts = [
      { id: 1001, name: '智能手机Pro', desc: '旗舰处理器', price: '2999', icon: 'success', bgColor: '#FF6B35', sold: '1.2万' },
      { id: 1002, name: '轻薄笔记本', desc: '超长续航', price: '4999', icon: 'info', bgColor: '#4A90D9', sold: '8900' },
      { id: 1003, name: '无线蓝牙耳机', desc: '降噪功能', price: '399', icon: 'warn', bgColor: '#52C41A', sold: '2.5万' },
      { id: 1004, name: '智能手表', desc: '健康监测', price: '899', icon: 'waiting', bgColor: '#FF69B4', sold: '1.8万' },
      { id: 1005, name: '机械键盘', desc: 'RGB背光', price: '299', icon: 'circle', bgColor: '#722ED1', sold: '6700' },
      { id: 1006, name: '游戏鼠标', desc: '16000DPI', price: '199', icon: 'success_no_circle', bgColor: '#FA541C', sold: '5400' },
      { id: 1007, name: '显示器4K', desc: 'IPS面板', price: '1299', icon: 'success', bgColor: '#FF6B35', sold: '4200' },
      { id: 1008, name: '无线路由器', desc: 'WiFi6', price: '399', icon: 'info', bgColor: '#4A90D9', sold: '3800' },
      { id: 1009, name: '便携充电宝', desc: '快充支持', price: '99', icon: 'warn', bgColor: '#52C41A', sold: '3.2万' },
      { id: 1010, name: '蓝牙音箱', desc: '高保真音质', price: '159', icon: 'waiting', bgColor: '#FF69B4', sold: '4500' }
    ];

    // 模糊搜索
    const results = allProducts.filter(p =>
      p.name.includes(keyword) || p.desc.includes(keyword)
    );

    this.setData({ results });
  },

  // 清空输入
  onClear: function() {
    this.setData({ keyword: '', hasSearched: false, results: [] });
  },

  // 取消
  onCancel: function() {
    wx.navigateBack();
  },

  // 清空历史
  onClearHistory: function() {
    wx.showModal({
      title: '确认清空',
      content: '确定要清空搜索历史吗？',
      success: (res) => {
        if (res.confirm) {
          this.setData({ searchHistory: [] });
          wx.setStorageSync('searchHistory', []);
        }
      }
    });
  },

  // 历史搜索点击
  onHistoryTap: function(e) {
    const keyword = e.currentTarget.dataset.keyword;
    this.setData({ keyword });
    this.performSearch(keyword);
  },

  // 热门搜索点击
  onHotTap: function(e) {
    const keyword = e.currentTarget.dataset.keyword;
    this.setData({ keyword });
    this.performSearch(keyword);
  },

  // 商品详情
  onProductTap: function(e) {
    const productId = e.currentTarget.dataset.id;
    wx.navigateTo({
      url: `/pages/detail/detail?id=${productId}`
    });
  },

  // 加入购物车
  onAddCart: function(e) {
    const product = e.currentTarget.dataset.product;
    app.addToCart(product, 1);
  }
});
