// 首页
Page({
  data: {
    hotProducts: [
      { id: 101, name: '无线蓝牙耳机', price: 199, image: '' },
      { id: 102, name: '智能手表', price: 599, image: '' },
      { id: 103, name: '便携充电宝', price: 129, image: '' },
      { id: 104, name: '机械键盘', price: 349, image: '' }
    ],
    newProducts: [
      { id: 201, name: '轻薄笔记本电脑', desc: '14英寸高性能', price: 4999 },
      { id: 202, name: '降噪耳机Pro', desc: '40小时续航', price: 899 },
      { id: 203, name: '智能音箱', desc: '语音助手', price: 299 }
    ]
  },

  onLoad: function() {
    console.log('🏠 首页加载');
  },

  onCategoryTap: function(e) {
    var id = e.currentTarget.dataset.id;
    console.log('📂 点击分类:', id);
    wx.switchTab({ url: '/pages/category/category' });
  },

  onProductTap: function(e) {
    var id = e.currentTarget.dataset.id;
    console.log('📦 查看商品:', id);
    wx.navigateTo({ url: '/pages/detail/detail?id=' + id });
  },

  onAddCart: function(e) {
    var product = e.currentTarget.dataset.product;
    console.log('🛒 加入购物车:', product.name);
    getApp().addToCart(product, 1);
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  onViewMore: function() {
    wx.switchTab({ url: '/pages/category/category' });
  }
});
