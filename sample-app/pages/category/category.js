// 分类页面
Page({
  data: {
    currentCategory: 0,
    products: [
      { id: 101, name: '无线耳机', price: 199, icon: 'success', color: '#FF6B35' },
      { id: 102, name: '充电器', price: 59, icon: 'info', color: '#4A90D9' },
      { id: 103, name: '数据线', price: 29, icon: 'waiting', color: '#52C41A' },
      { id: 104, name: '智能手表', price: 599, icon: 'success', color: '#FF6B35' },
      { id: 105, name: '智能音箱', price: 299, icon: 'info', color: '#4A90D9' }
    ],
    allProducts: {
      0: [
        { id: 101, name: '无线耳机', price: 199, icon: 'success', color: '#FF6B35' },
        { id: 102, name: '充电器', price: 59, icon: 'info', color: '#4A90D9' },
        { id: 103, name: '数据线', price: 29, icon: 'waiting', color: '#52C41A' }
      ],
      1: [
        { id: 201, name: 'T恤', price: 99, icon: 'success', color: '#4A90D9' },
        { id: 202, name: '衬衫', price: 159, icon: 'info', color: '#4A90D9' }
      ],
      2: [
        { id: 301, name: '面霜', price: 199, icon: 'success', color: '#FF69B4' },
        { id: 302, name: '精华', price: 299, icon: 'info', color: '#FF69B4' }
      ],
      3: [
        { id: 401, name: '坚果', price: 39, icon: 'success', color: '#52C41A' },
        { id: 402, name: '饼干', price: 19, icon: 'info', color: '#52C41A' }
      ],
      4: [
        { id: 501, name: '椅子', price: 299, icon: 'success', color: '#8B4513' },
        { id: 502, name: '桌子', price: 599, icon: 'info', color: '#8B4513' }
      ]
    }
  },

  onLoad: function() {
    console.log('📂 分类页加载');
  },

  onSelectCategory: function(e) {
    var index = e.currentTarget.dataset.index;
    console.log('📂 选择分类:', index);
    this.setData({
      currentCategory: index,
      products: this.data.allProducts[index] || []
    });
  },

  onProductTap: function(e) {
    var id = e.currentTarget.dataset.id;
    wx.navigateTo({ url: '/pages/detail/detail?id=' + id });
  },

  onAddCart: function(e) {
    var product = e.currentTarget.dataset.product;
    console.log('🛒 加入购物车:', product.name);
    getApp().addToCart(product, 1);
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  }
});
