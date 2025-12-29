// 商品详情页
Page({
  data: {
    product: {
      id: 101,
      name: '无线蓝牙耳机 Pro',
      desc: '高清音质 主动降噪 40小时超长续航',
      price: 199,
      originalPrice: 299,
      discount: '6.7折',
      sales: 2341,
      stock: 999,
      detail: '【产品特点】\n• 高清音质，还原真实声音\n• 主动降噪，沉浸式体验\n• 40小时超长续航\n• 蓝牙5.0，稳定连接\n• 轻量设计，佩戴舒适\n\n【包装清单】\n耳机 x 1\n充电线 x 1\n说明书 x 1\n收纳袋 x 1'
    },
    specs: [
      { id: 1, name: '黑色', selected: true },
      { id: 2, name: '白色', selected: false },
      { id: 3, name: '蓝色', selected: false }
    ],
    selectedSpec: '黑色',
    quantity: 1,
    cartCount: 0
  },

  onLoad: function(options) {
    console.log('📦 商品详情页加载, id:', options.id);
    this.updateCartCount();
  },

  onShow: function() {
    this.updateCartCount();
  },

  updateCartCount: function() {
    var app = getApp();
    this.setData({ cartCount: app.globalData.cartCount || 0 });
  },

  onSelectSpec: function(e) {
    var index = e.currentTarget.dataset.index;
    var specs = this.data.specs.map(function(spec, i) {
      spec.selected = (i === index);
      return spec;
    });
    var selected = specs[index].name;
    console.log('🎨 选择规格:', selected);
    this.setData({ specs: specs, selectedSpec: selected });
  },

  onIncrease: function() {
    var qty = this.data.quantity;
    if (qty < this.data.product.stock) {
      this.setData({ quantity: qty + 1 });
    }
  },

  onDecrease: function() {
    var qty = this.data.quantity;
    if (qty > 1) {
      this.setData({ quantity: qty - 1 });
    }
  },

  onAddCart: function() {
    var product = this.data.product;
    var quantity = this.data.quantity;
    console.log('🛒 加入购物车:', product.name, 'x', quantity);
    getApp().addToCart(product, quantity);
    this.updateCartCount();
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  onBuyNow: function() {
    console.log('💳 立即购买:', this.data.product.name, 'x', this.data.quantity);
    wx.showToast({ title: '订单创建成功', icon: 'success' });
  },

  onGoHome: function() {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onGoCart: function() {
    wx.switchTab({ url: '/pages/cart/cart' });
  }
});
