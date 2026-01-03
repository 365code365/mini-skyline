const app = getApp();

Page({
  data: {
    productId: null,
    product: {},
    cartCount: 0
  },

  onLoad: function(options) {
    const productId = parseInt(options.id);
    this.setData({ productId });
    this.loadProductDetail(productId);
  },

  onShow: function() {
    this.setData({ cartCount: app.globalData.cartCount });
  },

  // 加载商品详情
  loadProductDetail: function(productId) {
    // 模拟商品数据
    const products = {
      101: { id: 101, name: '无线蓝牙耳机', desc: '高保真音质，超长续航', price: '199', originalPrice: '399', icon: 'success', bgColor: '#FF6B35', tag: '限时特惠', stock: 999 },
      201: { id: 201, name: '智能手机Pro', desc: '旗舰处理器，拍照更清晰', price: '2999', originalPrice: '3499', icon: 'info', bgColor: '#4A90D9', tag: '新品上市', stock: 500 },
      1001: { id: 1001, name: '智能手机Pro', desc: '旗舰处理器，拍照更清晰', price: '2999', originalPrice: '3499', icon: 'success', bgColor: '#FF6B35', tag: '热销', stock: 500 }
    };

    const product = products[productId] || {
      id: productId,
      name: '精选商品',
      desc: '品质保证，值得信赖',
      price: '99',
      originalPrice: '199',
      icon: 'success',
      bgColor: '#FF6B35',
      tag: '新品',
      stock: 999
    };

    this.setData({ product });
  },

  // 联系客服
  onContact: function() {
    wx.showModal({
      title: '联系客服',
      content: '客服电话: 400-888-8888\n工作时间: 9:00-18:00',
      showCancel: false
    });
  },

  // 购物车
  onCart: function() {
    wx.switchTab({ url: '/pages/cart/cart' });
  },

  // 加入购物车
  onAddCart: function() {
    app.addToCart(this.data.product, 1);
    this.setData({ cartCount: app.globalData.cartCount });
  },

  // 立即购买
  onBuyNow: function() {
    const product = this.data.product;
    app.globalData.checkoutProducts = [{
      ...product,
      quantity: 1,
      selected: true
    }];
    app.globalData.checkoutTotal = product.price;

    wx.navigateTo({
      url: `/pages/address/address?from=checkout`
    });
  }
});
