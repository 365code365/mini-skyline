// 详情页
Page({
    data: {
        itemId: 0,
        price: 99,
        quantity: 1
    },
    
    onLoad: function(options) {
        __native_print('Detail page onLoad');
        if (options && options.id) {
            var id = parseInt(options.id);
            __native_print('Item ID: ' + id);
            this.setData({
                itemId: id,
                price: id * 99
            });
        }
    },
    
    onShow: function() {
        __native_print('Detail page onShow');
    },
    
    // 返回上一页
    onBack: function() {
        __native_print('Navigate back');
        wx.navigateBack();
    },
    
    // 减少数量
    onDecrease: function() {
        if (this.data.quantity > 1) {
            this.setData({ quantity: this.data.quantity - 1 });
            __native_print('Quantity: ' + this.data.quantity);
        }
    },
    
    // 增加数量
    onIncrease: function() {
        this.setData({ quantity: this.data.quantity + 1 });
        __native_print('Quantity: ' + this.data.quantity);
    },
    
    // 加入购物车
    onAddCart: function() {
        __native_print('Add to cart: item ' + this.data.itemId + ', qty ' + this.data.quantity);
        wx.showToast({
            title: '已加入购物车',
            icon: 'success'
        });
    },
    
    // 立即购买
    onBuyNow: function() {
        __native_print('Buy now: item ' + this.data.itemId + ', qty ' + this.data.quantity);
        wx.showToast({
            title: '购买成功',
            icon: 'success'
        });
    }
});
