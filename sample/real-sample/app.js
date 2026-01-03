// 精选商城小程序
App({
  globalData: {
    userInfo: {
      nickName: '用户',
      avatar: '',
      level: '普通会员'
    },
    cart: [],
    cartCount: 0,
    orders: [],
    addresses: [
      {
        id: 1,
        name: '张三',
        phone: '138****8888',
        address: '北京市朝阳区建国路88号',
        isDefault: true
      }
    ]
  },

  onLaunch: function() {
    console.log('🛒 精选商城启动');
    this.loadFromStorage();
  },

  // 从本地存储加载数据
  loadFromStorage: function() {
    try {
      var cart = wx.getStorageSync('cart');
      if (cart) {
        this.globalData.cart = cart;
        this.updateCartCount();
      }

      var orders = wx.getStorageSync('orders');
      if (orders) {
        this.globalData.orders = orders;
      }
    } catch (e) {
      console.error('加载本地数据失败', e);
    }
  },

  // 保存数据到本地存储
  saveToStorage: function() {
    try {
      wx.setStorageSync('cart', this.globalData.cart);
      wx.setStorageSync('orders', this.globalData.orders);
    } catch (e) {
      console.error('保存数据失败', e);
    }
  },

  // 添加商品到购物车
  addToCart: function(product, quantity) {
    quantity = quantity || 1;
    var cart = this.globalData.cart;
    var found = false;

    for (var i = 0; i < cart.length; i++) {
      if (cart[i].id === product.id) {
        cart[i].quantity += quantity;
        found = true;
        break;
      }
    }

    if (!found) {
      cart.push({
        id: product.id,
        name: product.name,
        price: product.price,
        image: product.image,
        quantity: quantity,
        selected: true
      });
    }

    this.updateCartCount();
    this.saveToStorage();
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  // 更新购物车数量
  updateCartCount: function() {
    var count = 0;
    this.globalData.cart.forEach(function(item) {
      count += item.quantity;
    });
    this.globalData.cartCount = count;
  },

  // 获取购物车总价
  getCartTotal: function() {
    var total = 0;
    this.globalData.cart.forEach(function(item) {
      if (item.selected) {
        total += item.price * item.quantity;
      }
    });
    return total.toFixed(2);
  },

  // 获取选中的商品数量
  getSelectedCount: function() {
    var count = 0;
    this.globalData.cart.forEach(function(item) {
      if (item.selected) {
        count += item.quantity;
      }
    });
    return count;
  },

  // 创建订单
  createOrder: function(address, products, total) {
    var order = {
      id: Date.now(),
      orderNo: 'ORD' + Date.now(),
      products: products,
      total: total,
      address: address,
      status: 'pending',
      statusText: '待付款',
      createTime: new Date().toLocaleString()
    };

    this.globalData.orders.unshift(order);
    this.saveToStorage();
    return order;
  }
});
