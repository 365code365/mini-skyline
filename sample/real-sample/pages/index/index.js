const app = getApp();

Page({
  data: {
    banners: [
      { id: 1, tag: '限时特惠', title: '新品上市 全场8折', desc: '精选好物 品质保证', color: '#FF6B35' },
      { id: 2, tag: '会员专享', title: '会员日 特惠狂欢', desc: '尊享折扣 诚意满满', color: '#4A90D9' },
      { id: 3, tag: '清仓大促', title: '清仓甩卖 低至5折', desc: '数量有限 先到先得', color: '#52C41A' }
    ],
    categories: [
      { id: 1, name: '数码', icon: 'success', color: '#FF6B35' },
      { id: 2, name: '服饰', icon: 'info', color: '#4A90D9' },
      { id: 3, name: '美妆', icon: 'warn', color: '#FF69B4' },
      { id: 4, name: '食品', icon: 'waiting', color: '#52C41A' },
      { id: 5, name: '家居', icon: 'circle', color: '#722ED1' },
      { id: 6, name: '母婴', icon: 'success_no_circle', color: '#FA541C' }
    ],
    flashProducts: [
      { id: 101, name: '无线蓝牙耳机', price: '199', originalPrice: '399', icon: 'success', bgColor: '#FF6B35', badge: '限时' },
      { id: 102, name: '智能手表', price: '499', originalPrice: '899', icon: 'info', bgColor: '#4A90D9', badge: '热卖' },
      { id: 103, name: '便携充电宝', price: '99', originalPrice: '199', icon: 'warn', bgColor: '#52C41A' },
      { id: 104, name: '蓝牙音箱', price: '159', originalPrice: '299', icon: 'waiting', bgColor: '#722ED1' }
    ],
    hotProducts: [
      { id: 201, name: '智能手机Pro', desc: '旗舰处理器', price: '2999', sold: '1.2万', icon: 'success', bgColor: '#FF6B35', badge: 'TOP1' },
      { id: 202, name: '轻薄笔记本', desc: '超长续航', price: '4999', sold: '8900', icon: 'info', bgColor: '#4A90D9' },
      { id: 203, name: '机械键盘', desc: 'RGB背光', price: '299', sold: '6700', icon: 'warn', bgColor: '#52C41A', badge: '新品' },
      { id: 204, name: '游戏鼠标', desc: '16000DPI', price: '199', sold: '5400', icon: 'waiting', bgColor: '#FF69B4' },
      { id: 205, name: '显示器4K', desc: 'IPS面板', price: '1299', sold: '4200', icon: 'circle', bgColor: '#722ED1' },
      { id: 206, name: '无线路由器', desc: 'WiFi6', price: '399', sold: '3800', icon: 'success_no_circle', bgColor: '#FA541C' }
    ],
    newProducts: [
      { id: 301, name: '智能扫地机器人', desc: '全自动清扫', price: '1299', originalPrice: '1999', tag: '新品上市', icon: 'success', bgColor: '#FF6B35' },
      { id: 302, name: '空气净化器', desc: '去除甲醛', price: '899', originalPrice: '1499', tag: '热销', icon: 'info', bgColor: '#4A90D9' },
      { id: 303, name: '智能门锁', desc: '指纹识别', price: '1599', originalPrice: '2499', icon: 'warn', bgColor: '#52C41A' },
      { id: 304, name: '智能窗帘', desc: '语音控制', price: '799', originalPrice: '1299', tag: '推荐', icon: 'waiting', bgColor: '#FF69B4' },
      { id: 305, name: '智能灯泡', desc: '1600万色', price: '99', originalPrice: '199', icon: 'circle', bgColor: '#722ED1' },
      { id: 306, name: '智能插座', desc: '远程控制', price: '59', originalPrice: '99', tag: '特价', icon: 'success_no_circle', bgColor: '#FA541C' }
    ]
  },

  onLoad: function(options) {
    console.log('首页加载');
  },

  onShow: function() {
    // 页面显示时可以刷新数据
  },

  // 跳转搜索
  onSearchTap: function() {
    wx.navigateTo({ url: '/pages/search/search' });
  },

  // 分类点击
  onCategoryTap: function(e) {
    const categoryId = e.currentTarget.dataset.id;
    wx.navigateTo({
      url: `/pages/category/category?id=${categoryId}`
    });
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
  },

  // 查看全部
  onViewAll: function() {
    wx.switchTab({ url: '/pages/category/category' });
  }
});
