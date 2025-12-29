// 商城小程序
App({
  globalData: {
    userInfo: null,
    cart: [],
    cartCount: 0
  },
  
  onLaunch: function() {
    console.log('🛒 Mini Shop 启动');
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
        quantity: quantity
      });
    }
    
    this.updateCartCount();
    console.log('🛒 添加到购物车:', product.name, 'x', quantity);
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
      total += item.price * item.quantity;
    });
    return total.toFixed(2);
  }
});
