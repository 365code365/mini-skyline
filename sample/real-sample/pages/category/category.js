const app = getApp();

Page({
  data: {
    activeCategory: 1,
    scrollTop: 0,
    categories: [
      { id: 1, name: '数码' },
      { id: 2, name: '服饰' },
      { id: 3, name: '美妆' },
      { id: 4, name: '食品' },
      { id: 5, name: '家居' },
      { id: 6, name: '母婴' },
      { id: 7, name: '运动' },
      { id: 8, name: '图书' },
      { id: 9, name: '汽车' },
      { id: 10, name: '虚拟' }
    ],
    subcategories: [],
    products: []
  },

  onLoad: function(options) {
    const categoryId = options.id ? parseInt(options.id) : 1;
    this.setData({ activeCategory: categoryId });
    this.loadCategoryData(categoryId);
  },

  // 切换分类
  onCategoryChange: function(e) {
    const categoryId = parseInt(e.currentTarget.dataset.id);
    this.setData({ activeCategory: categoryId, scrollTop: 0 });
    this.loadCategoryData(categoryId);
  },

  // 加载分类数据
  loadCategoryData: function(categoryId) {
    // 模拟加载子分类
    const subcategories = this.getSubcategories(categoryId);
    // 模拟加载商品
    const products = this.getProducts(categoryId);

    this.setData({
      subcategories: subcategories,
      products: products
    });
  },

  // 获取子分类
  getSubcategories: function(categoryId) {
    const subData = {
      1: [
        { id: 101, name: '手机', icon: 'success', color: '#FF6B35' },
        { id: 102, name: '电脑', icon: 'info', color: '#4A90D9' },
        { id: 103, name: '耳机', icon: 'warn', color: '#52C41A' },
        { id: 104, name: '相机', icon: 'waiting', color: '#FF69B4' },
        { id: 105, name: '平板', icon: 'circle', color: '#722ED1' }
      ],
      2: [
        { id: 201, name: '男装', icon: 'success', color: '#FF6B35' },
        { id: 202, name: '女装', icon: 'info', color: '#4A90D9' },
        { id: 203, name: '鞋靴', icon: 'warn', color: '#52C41A' },
        { id: 204, name: '箱包', icon: 'waiting', color: '#FF69B4' }
      ],
      3: [
        { id: 301, name: '护肤', icon: 'success', color: '#FF6B35' },
        { id: 302, name: '彩妆', icon: 'info', color: '#4A90D9' },
        { id: 303, name: '香水', icon: 'warn', color: '#52C41A' }
      ]
    };
    return subData[categoryId] || [];
  },

  // 获取商品
  getProducts: function(categoryId) {
    const productData = {
      1: [
        { id: 1001, name: '智能手机Pro', desc: '旗舰处理器', price: '2999', icon: 'success', bgColor: '#FF6B35' },
        { id: 1002, name: '轻薄笔记本', desc: '超长续航', price: '4999', icon: 'info', bgColor: '#4A90D9' },
        { id: 1003, name: '无线蓝牙耳机', desc: '降噪功能', price: '399', icon: 'warn', bgColor: '#52C41A' },
        { id: 1004, name: '智能手表', desc: '健康监测', price: '899', icon: 'waiting', bgColor: '#FF69B4' },
        { id: 1005, name: '机械键盘', desc: 'RGB背光', price: '299', icon: 'circle', bgColor: '#722ED1' },
        { id: 1006, name: '游戏鼠标', desc: '16000DPI', price: '199', icon: 'success_no_circle', bgColor: '#FA541C' },
        { id: 1007, name: '显示器4K', desc: 'IPS面板', price: '1299', icon: 'success', bgColor: '#FF6B35' },
        { id: 1008, name: '无线路由器', desc: 'WiFi6', price: '399', icon: 'info', bgColor: '#4A90D9' }
      ],
      2: [
        { id: 2001, name: '男士T恤', desc: '纯棉舒适', price: '99', icon: 'success', bgColor: '#FF6B35' },
        { id: 2002, name: '休闲牛仔裤', desc: '修身版型', price: '199', icon: 'info', bgColor: '#4A90D9' },
        { id: 2003, name: '连衣裙', desc: '优雅气质', price: '299', icon: 'warn', bgColor: '#52C41A' },
        { id: 2004, name: '运动鞋', desc: '透气轻便', price: '399', icon: 'waiting', bgColor: '#FF69B4' },
        { id: 2005, name: '双肩包', desc: '大容量', price: '199', icon: 'circle', bgColor: '#722ED1' }
      ],
      3: [
        { id: 3001, name: '洁面乳', desc: '温和清洁', price: '89', icon: 'success', bgColor: '#FF6B35' },
        { id: 3002, name: '精华液', desc: '补水保湿', price: '199', icon: 'info', bgColor: '#4A90D9' },
        { id: 3003, name: '口红套装', desc: '多种色号', price: '299', icon: 'warn', bgColor: '#52C41A' }
      ],
      4: [
        { id: 4001, name: '进口零食', desc: '美味可口', price: '49', icon: 'success', bgColor: '#FF6B35' },
        { id: 4002, name: '坚果礼盒', desc: '品质保证', price: '99', icon: 'info', bgColor: '#4A90D9' }
      ],
      5: [
        { id: 5001, name: '智能台灯', desc: '护眼设计', price: '199', icon: 'success', bgColor: '#FF6B35' },
        { id: 5002, name: '床上四件套', desc: '纯棉亲肤', price: '299', icon: 'info', bgColor: '#4A90D9' }
      ]
    };
    return productData[categoryId] || [];
  },

  // 子分类点击
  onSubCategoryTap: function(e) {
    const subId = e.currentTarget.dataset.id;
    wx.showToast({ title: '加载中...', icon: 'loading' });
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
